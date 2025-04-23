mod lifecycle;
mod streaming;
mod upload;

use std::time::Duration;

use crate::{
    client::{LiveViewClientConfiguration, LogLevel, Platform},
    dom::Document,
    LiveViewClient,
};

macro_rules! json_payload {
    ($json:tt) => {{
        let val = serde_json::json!($json);
        phoenix_channels_client::Payload::JSONPayload { json: val.into() }
    }};
}

#[macro_export]
macro_rules! expect_status_matches {
    ($watcher:expr, $pattern:pat, $timeout_secs:expr) => {{
        use std::time::Duration;

        use tokio::time::timeout;
        let fut = $watcher.wait_for(|s| matches!(s, $pattern));
        timeout(Duration::from_secs($timeout_secs), fut)
            .await
            .expect(&format!(
                "timed out after {}s waiting for status to match pattern",
                $timeout_secs
            ))
            .expect("status watcher was dropped or encountered an error")
    }};
    ($watcher:expr, $pattern:pat) => {
        expect_status_matches!($watcher, $pattern, 2)
    };
}

#[macro_export]
macro_rules! expect_status_with {
    ($watcher:expr, $predicate:expr, $timeout_secs:expr) => {{
        let fut = $watcher.wait_for($predicate);
        timeout(Duration::from_secs($timeout_secs), fut)
            .await
            .expect(&format!(
                "timed out after {}s waiting for status predicate to match",
                $timeout_secs
            ))
            .expect("status watcher was dropped or encountered an error")
    }};
    ($watcher:expr, $predicate:expr) => {
        expect_status_with!($watcher, $predicate, 2)
    };
}

pub(crate) use json_payload;
use tokio::time::timeout;

macro_rules! assert_doc_eq {
    ($gold:expr, $test:expr) => {{
        use pretty_assertions::assert_eq;
        let gold = Document::parse($gold).expect("Gold document failed to parse");
        let test = Document::parse($test).expect("Test document failed to parse");
        assert_eq!(gold.to_string(), test.to_string());
    }};
}

#[cfg(target_os = "android")]
const HOST: &str = "10.0.2.2:4001";

#[cfg(not(target_os = "android"))]
const HOST: &str = "127.0.0.1:4001";

#[tokio::test]
async fn test_basic_connection() {
    let url = format!("http://{HOST}/hello");
    let mut config = LiveViewClientConfiguration::default();
    config.log_level = LogLevel::Debug;
    config.format = Platform::Swiftui;
    let client = LiveViewClient::initial_connect(config, url, Default::default())
        .await
        .expect("Failed to create client");

    let mut watcher = client.watch_status();
    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    let initial_doc = client.document().expect("Failed to get initial page");

    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        Hello SwiftUI!
    </Text>
</VStack>"#;

    assert_doc_eq!(expected, initial_doc.to_string());

    let url = format!("http://{HOST}/hello");

    client
        .reconnect(url, Default::default(), None)
        .expect("reconnect failed");

    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    let initial_doc = client.document().expect("Failed to get initial page");

    assert_doc_eq!(expected, initial_doc.to_string());

    let url = format!("http://{HOST}/nav/first_page");

    client
        .reconnect(url.clone(), Default::default(), None)
        .expect("reconnect failed");

    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connecting);
    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    let doc = client.document().expect("Failed to get document");
    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        first_page
    </Text>
    <NavigationLink id="Next" destination="/nav/next">
        <Text>
            NEXT
        </Text>
    </NavigationLink>
</VStack>"#;
    assert_doc_eq!(expected, doc.to_string());

    client.disconnect().await.expect("disconnect failed");

    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Disconnected);

    client
        .reconnect(url, Default::default(), None)
        .expect("reconnect failed");

    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    assert_doc_eq!(expected, doc.to_string());
}

#[tokio::test]
async fn test_style_urls() {
    let url = format!("http://{HOST}/hello");
    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;

    let client = LiveViewClient::initial_connect(config, url, Default::default())
        .await
        .expect("Failed to create client");

    let style_urls = client.style_urls().expect("Failed to get style URLs");
    let expected_style_urls = vec!["/assets/app.swiftui.styles".to_string()];

    assert_eq!(style_urls, expected_style_urls);
}

// Navigation tests
#[tokio::test]
async fn test_basic_navigation() {
    let url = format!("http://{HOST}/nav/first_page");
    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;

    let client = LiveViewClient::initial_connect(config, url, Default::default())
        .await
        .expect("Failed to create client");

    let mut watcher = client.watch_status();

    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    let doc = client.document().expect("Failed to get document");
    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        first_page
    </Text>
    <NavigationLink id="Next" destination="/nav/next">
        <Text>
            NEXT
        </Text>
    </NavigationLink>
</VStack>"#;
    assert_doc_eq!(expected, doc.to_string());

    // Navigate to second page
    let next_url = format!("http://{HOST}/nav/second_page");
    client
        .navigate(next_url.clone(), Default::default())
        .expect("Failed to navigate");

    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        second_page
    </Text>
    <NavigationLink id="Next" destination="/nav/next">
        <Text>
            NEXT
        </Text>
    </NavigationLink>
</VStack>"#;
    let expected = Document::parse(expected).unwrap();

    expect_status_with!(watcher, |status| {
        if let crate::client::inner::ClientStatus::Connected(con) = status {
            expected.to_string() == con.document.render()
        } else {
            false
        }
    });
}

#[tokio::test]
async fn test_back_and_forward_navigation() {
    let url = format!("http://{HOST}/nav/first_page");
    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;

    let client = LiveViewClient::initial_connect(config, url, Default::default())
        .await
        .expect("Failed to create client");

    let mut watcher = client.watch_status();

    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    // Verify initial page
    let doc = client.document().expect("Failed to get document");
    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        first_page
    </Text>
    <NavigationLink id="Next" destination="/nav/next">
        <Text>
            NEXT
        </Text>
    </NavigationLink>
</VStack>"#;

    assert_doc_eq!(expected, doc.to_string());

    let second_url = format!("http://{HOST}/nav/second_page");
    client
        .navigate(second_url, Default::default())
        .expect("Failed to navigate");

    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        second_page
    </Text>
    <NavigationLink id="Next" destination="/nav/next">
        <Text>
            NEXT
        </Text>
    </NavigationLink>
</VStack>"#;

    let expected = Document::parse(expected).unwrap();

    expect_status_with!(watcher, |status| {
        if let crate::client::inner::ClientStatus::Connected(con) = status {
            expected.to_string() == con.document.render()
        } else {
            false
        }
    });

    let third_url = format!("http://{HOST}/nav/third_page");
    client
        .navigate(third_url, Default::default())
        .expect("Failed to navigate");

    assert!(client.can_go_back(), "Back navigation impossible");

    client
        .back(Default::default())
        .expect("Failed to navigate back");

    // Verify we're back on second page
    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        second_page
    </Text>
    <NavigationLink id="Next" destination="/nav/next">
        <Text>
            NEXT
        </Text>
    </NavigationLink>
</VStack>"#;

    let expected = Document::parse(expected).unwrap();

    expect_status_with!(watcher, |status| {
        if let crate::client::inner::ClientStatus::Connected(con) = status {
            expected.to_string() == con.document.render()
        } else {
            false
        }
    });

    assert!(client.can_go_forward(), "Forward navigation impossible");
    client
        .forward(Default::default())
        .expect("Failed to navigate forward");

    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        third_page
    </Text>
    <NavigationLink id="Next" destination="/nav/next">
        <Text>
            NEXT
        </Text>
    </NavigationLink>
</VStack>"#;

    let expected = Document::parse(expected).unwrap();

    expect_status_with!(watcher, |status| {
        if let crate::client::inner::ClientStatus::Connected(con) = status {
            expected.to_string() == con.document.render()
        } else {
            false
        }
    });
}

#[tokio::test]
async fn thermostat_click() {
    let url = format!("http://{HOST}/thermostat");

    let config = LiveViewClientConfiguration {
        format: Platform::Swiftui,
        ..Default::default()
    };

    let client = LiveViewClient::initial_connect(config, url, Default::default())
        .await
        .expect("Failed to create client");

    let mut watcher = client.watch_status();
    expect_status_matches!(watcher, crate::client::inner::ClientStatus::Connected(_));

    let initial_doc = client.document().expect("Failed to get initial page");

    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        Current temperature: 70°F
    </Text>
    <Button phx-click="inc_temperature">
        +
    </Button>
</VStack>"#;

    assert_doc_eq!(expected, initial_doc.to_string());

    let payload = json_payload!({"type": "click", "event": "inc_temperature", "value": {}});
    client
        .call("event".into(), payload)
        .await
        .expect("error on click");

    let expected = r#"
    <Group id="flash-group" />
    <VStack>
        <Text>
            Current temperature: 71°F
        </Text>
        <Button phx-click="inc_temperature">
            +
        </Button>
    </VStack>"#;

    assert_doc_eq!(expected, initial_doc.to_string());
}
