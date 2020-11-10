use crossterm::event::{
    poll, read, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use log::warn;
use std::error::Error;
use std::io::{stdout, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use tui::backend::CrosstermBackend;
use tui::Terminal;

use crate::handlers::handle_event;
use crate::kafka::KafkaWrapper;
use crate::model::{
    ClusterInfo, Event, GroupInfo, OffsetAndMetadata, OffsetValue, TopicDetail, TopicInfo,
};
use crate::offsets_consumer::OffsetsConsumer;

use crate::app::Context::{TopicDetailPage, TopicListPage};
use crate::config::Config;
use crate::ui;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tui::widgets::TableState;

pub enum Context {
    TopicListPage,
    TopicDetailPage,
}

pub struct App {
    pub message: String,
    kafka_wrapper: KafkaWrapper,
    pub topic_table_state: TableState,
    pub context: Context,
    pub cluster_info: ClusterInfo,
    pub topic_infos: Vec<TopicInfo>,
    pub group_infos: Vec<GroupInfo>,
    pub selected_topic: Option<String>,
    pub topic_detail: Option<TopicDetail>,
    pub offsets: Arc<Mutex<HashMap<OffsetAndMetadata, OffsetValue>>>,
}

impl App {
    pub fn new(config: &Config) -> App {
        let kafka_wrapper = KafkaWrapper::new(config.brokers.as_ref());
        let cluster_info = kafka_wrapper.get_cluster_infos();
        let topic_infos = kafka_wrapper.get_topic_infos();
        let group_infos = kafka_wrapper.get_group_infos();
        let offsets = Arc::new(Mutex::new(HashMap::new()));

        App {
            message: String::from("Welcome"),
            kafka_wrapper,
            topic_table_state: TableState::default(),
            context: TopicListPage,
            cluster_info,
            topic_infos,
            group_infos,
            selected_topic: None,
            topic_detail: None,
            offsets,
        }
    }

    fn load_topic_list(&mut self) {
        self.cluster_info = self.kafka_wrapper.get_cluster_infos();
        self.topic_infos = self.kafka_wrapper.get_topic_infos()
    }

    pub fn load_topic_detail(&mut self) {
        let topic = self.selected_topic.as_ref().unwrap();
        self.topic_detail = self.kafka_wrapper.get_topic_detail(topic.as_str());
    }

    pub fn change_message(&mut self, new_message: String) {
        self.message = new_message;
    }

    pub fn select_next_topic(&mut self) {
        let i = match self.topic_table_state.selected() {
            Some(i) => {
                if i >= self.topic_infos.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.topic_table_state.select(Some(i));
    }

    pub fn select_previous_topic(&mut self) {
        let i = match self.topic_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.topic_infos.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.topic_table_state.select(Some(i));
    }

    pub fn switch_context(&mut self, context: Context) {
        match context {
            TopicListPage => self.load_topic_list(),
            TopicDetailPage => self.load_topic_detail(),
        }
        self.context = context
    }

    pub fn select_current_topic(&mut self) {
        self.selected_topic = self.get_selected_topic().map(|s| s.to_string());
        self.switch_context(TopicDetailPage)
    }

    fn get_selected_topic(&self) -> Option<&str> {
        self.topic_table_state
            .selected()
            .map(|i| self.topic_infos[i].name.borrow())
    }
}

#[tokio::main]
pub async fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let mut app = App::new(&config);

    // Definition of the event channel. An event is triggered by tick time or by a user keyboard
    // input
    let (tx, rx) = mpsc::channel();

    //TODO tick-rate from conf
    let tick_rate = Duration::from_millis(5000);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        loop {
            if poll(timeout).unwrap() {
                match read().unwrap() {
                    CEvent::Key(event) => tx.send(Event::Input(event.code)).unwrap(),
                    _ => warn!("other event"),
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    // Terminal tuning for crossterm
    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Spawn a thread for the offsets consumer
    let db = Arc::clone(&app.offsets);
    tokio::spawn(OffsetsConsumer::get_consumer_group_offsets(
        Arc::new(config.brokers),
        db,
    ));

    // app loop. Wait for some event, then draw the terminal
    loop {
        terminal.draw(|f| match &app.context {
            TopicListPage => ui::draw(f, &mut app),
            TopicDetailPage => ui::draw_topic_detail(f, &app),
        })?;

        let event = rx.recv()?;
        match event {
            Event::Input(key) => match key {
                // If the user use 'q', quit the app, else redirect the events to the current
                // context page.
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    break;
                }
                _ => handle_event(event, &mut app),
            },
            Event::Tick => handle_event(event, &mut app),
        }
    }
    Ok(())
}
