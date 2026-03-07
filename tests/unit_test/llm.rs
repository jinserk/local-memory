use local_memory::config::{Config, ExtractorConfig, ExtractorProvider};
use local_memory::model::get_llm_provider;

#[test]
fn test_get_llm_provider_ollama_config() {
    let mut config = Config::default();
    config.llm_extractor = Some(ExtractorConfig {
        provider: ExtractorProvider::Ollama,
        name: "llama3".to_string(),
        api_key: None,
        base_url: Some("http://localhost:11434".to_string()),
        auto_download: true,
    });
    
    let provider = get_llm_provider(&config);
    assert!(provider.is_some());
    assert_eq!(provider.unwrap().model(), "llama3");
}
