use crate::{
    client::{LiveViewClientConfiguration, Platform},
    dom::Document,
    LiveViewClient,
};

macro_rules! assert_doc_eq {
    ($gold:expr, $test:expr) => {
        let gold = Document::parse($gold).expect("Gold document failed to parse");
        let test = Document::parse($test).expect("Test document failed to parse");
        assert_eq!(gold.to_string(), test.to_string());
    };
}

#[cfg(target_os = "android")]
const HOST: &str = "10.0.2.2:4001";

#[cfg(not(target_os = "android"))]
const HOST: &str = "127.0.0.1:4001";

#[tokio::test]
async fn test_basic_connection() {
    let url = format!("http://{HOST}/hello");
    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;
    let client = LiveViewClient::initial_connect(config, url)
        .await
        .expect("Failed to create client");

    let initial_doc = client.document().expect("Failed to get initial page");

    let expected = r#"
<Group id="flash-group" />
<VStack>
    <Text>
        Hello SwiftUI!
    </Text>
</VStack>"#;

    assert_doc_eq!(expected, initial_doc.to_string());
}

#[tokio::test]
async fn test_style_urls() {
    let url = format!("http://{HOST}/hello");
    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;

    let client = LiveViewClient::initial_connect(config, url)
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

    let client = LiveViewClient::initial_connect(config, url)
        .await
        .expect("Failed to create client");

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
        .navigate(next_url, Default::default())
        .await
        .expect("Failed to navigate");

    let doc = client.document().expect("Failed to get document");
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
    assert_doc_eq!(expected, doc.to_string());
}

#[tokio::test]
async fn test_back_and_forward_navigation() {
    let url = format!("http://{HOST}/nav/first_page");
    let mut config = LiveViewClientConfiguration::default();
    config.format = Platform::Swiftui;

    let client = LiveViewClient::initial_connect(config, url)
        .await
        .expect("Failed to create client");

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
        .await
        .expect("Failed to navigate");

    let doc = client.document().expect("Failed to get document");
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
    assert_doc_eq!(expected, doc.to_string());

    let third_url = format!("http://{HOST}/nav/third_page");
    client
        .navigate(third_url, Default::default())
        .await
        .expect("Failed to navigate");

    assert!(client.can_go_back(), "Back navigation impossible");

    client.back(None).await.expect("Failed to navigate back");

    // Verify we're back on second page
    let doc = client.document().expect("Failed to get document");
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
    assert_doc_eq!(expected, doc.to_string());

    // Test forward navigation
    assert!(client
        .can_go_forward()
        .expect("Failed to check forward navigation"));
    client
        .forward(None)
        .await
        .expect("Failed to navigate forward");

    let doc = client.document().expect("Failed to get document");
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
    assert_doc_eq!(expected, doc.to_string());
}
