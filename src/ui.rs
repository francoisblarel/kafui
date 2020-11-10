use crate::app::App;
use crate::model::OffsetAndMetadata::OffsetKey;
use crate::model::TopicDetail;
use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph, Row, Table, Wrap};
use tui::Frame;

macro_rules! span_bold {
    ($text: expr) => {
        Span::styled($text, Style::default().add_modifier(Modifier::BOLD))
    };
}

pub fn draw<B: Backend>(backend: &mut Frame<B>, appli: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(70),
                Constraint::Percentage(10),
            ]
            .as_ref(),
        )
        .split(backend.size());

    draw_cluster_infos(backend, appli, chunks[0]);
    draw_topic_infos(backend, appli, chunks[1]);

    let block = Block::default().title("WOW").borders(Borders::ALL);
    backend.render_widget(block, chunks[2])
}

pub fn draw_topic_detail<B: Backend>(backend: &mut Frame<B>, app: &App) {
    let selected_topic = app.selected_topic.as_ref().unwrap();
    let topic_detail: &TopicDetail = app.topic_detail.as_ref().unwrap();
    let offsets_map = app.offsets.lock().unwrap();

    let groups = &app.group_infos;
    let consumers: Vec<_> = groups
        .iter()
        .filter(|g| g.consume_topic(selected_topic.as_str()))
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(backend.size());

    let block = Block::default()
        .title(format!("Topic {:?}", selected_topic))
        .borders(Borders::ALL);

    let mut topic_infos = vec![
        Spans::from("Topic infos"),
        Spans::from(""),
        Spans::from(vec![
            Span::from("name : "),
            Span::from(topic_detail.info.name.to_owned()),
        ]),
        Spans::from(vec![
            Span::from("partitions : "),
            Span::from(topic_detail.info.nb_partitions.to_string()),
        ]),
        Spans::from(vec![
            Span::from("nb de messages : "),
            Span::from(topic_detail.message_count.to_string()),
        ]),
    ];
    topic_infos.push(Spans::from("\n"));
    for offset in &topic_detail.offsets {
        topic_infos.push(Spans::from(vec![
            Span::from("\npartition :"),
            Span::from(offset.id.to_string()),
            Span::from("  nb de messages : "),
            Span::from(offset.count.to_string()),
            Span::from("  offset de "),
            Span::from(offset.low.to_string()),
            Span::from(" a "),
            Span::from(offset.high.to_string()),
            Span::from("  leader :"),
            Span::from(offset.leader.to_string()),
        ]))
    }
    topic_infos.push(Spans::from("\n"));
    for consumer in consumers {
        let mut consumer_lag = 0;

        let mut details: Vec<Spans> = vec![];
        for partition in &topic_detail.offsets {
            let key = OffsetKey {
                group: consumer.name.to_string(),
                topic: topic_detail.info.name.to_string(),
                partition: partition.id,
            };
            let offset = offsets_map.get(&key).map(|v| v.offset).unwrap_or_else(|| 0);
            let partition_lag = partition.high - offset;
            consumer_lag += partition_lag;
            let group_partition_detail = Spans::from(vec![Span::from(format!(
                "\npartition {} : {} (lag = {} )",
                partition.id, offset, partition_lag
            ))]);
            details.push(group_partition_detail);
        }

        topic_infos.push(Spans::from(vec![
            Span::from("\n"),
            Span::from("\nconsumer : "),
            Span::from(consumer.name.as_str()),
            Span::from(format!("(state={})", consumer.state.as_str())),
            Span::from(" lag ="),
            Span::from(consumer_lag.to_string()),
        ]));
        topic_infos.push(Spans::from("\noffsets: \n"));
        details.iter().for_each(|d| topic_infos.push(d.to_owned()))
    }
    let paragraph = Paragraph::new(topic_infos).block(block);
    backend.render_widget(paragraph, chunks[0])
}

fn draw_cluster_infos<B: Backend>(backend: &mut Frame<B>, app: &mut App, area: Rect) {
    let ci = &app.cluster_info;
    let cluster_block = Block::default()
        .title(Span::styled(
            "Cluster infos",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL);
    let cluster_infos = vec![
        Spans::from(format!("message: {}", app.message)),
        Spans::from("Here are the kafka cluster infos"),
        Spans::from(""),
        Spans::from(vec![
            span_bold!("broker id :"),
            Span::from(ci.broker_id.to_string()),
        ]),
        Spans::from(vec![
            span_bold!("broker name :"),
            Span::from(ci.broker_name.to_string()),
        ]),
        Spans::from(vec![
            span_bold!("broker count :"),
            Span::from(ci.broker_count.to_string()),
        ]),
        Spans::from(vec![
            span_bold!("topic count :"),
            Span::from(ci.topic_count.to_string()),
        ]),
    ];
    let paragraph = Paragraph::new(cluster_infos)
        .block(cluster_block)
        .wrap(Wrap { trim: true });
    backend.render_widget(paragraph, area);
}

fn draw_topic_infos<B: Backend>(backend: &mut Frame<B>, appli: &mut App, area: Rect) {
    let topics = &appli.topic_infos;
    let headers = ["name", "partitions nb"];

    let values: Vec<Vec<String>> = topics
        .iter()
        .map(|ti| vec![ti.name.to_owned(), ti.nb_partitions.to_string()])
        .collect();
    let rows = values.iter().map(|top| Row::Data(top.iter()));

    let block = Block::default().title("Topics").borders(Borders::ALL);

    let selected_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let table = Table::new(headers.iter(), rows)
        .block(block)
        .widths(&[
            Constraint::Percentage(50),
            Constraint::Length(30),
            Constraint::Max(10),
        ])
        .highlight_style(selected_style)
        .highlight_symbol(">> ");

    backend.render_stateful_widget(table, area, &mut appli.topic_table_state);
}
