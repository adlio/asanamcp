//! HTTP client for the Asana API.

use serde::de::DeserializeOwned;

use crate::types::{DataWrapper, ListWrapper};
use crate::Error;

const BASE_URL: &str = "https://app.asana.com/api/1.0";
const ENV_VAR: &str = "ASANA_TOKEN";

/// Client for interacting with the Asana API.
#[derive(Debug, Clone)]
pub struct AsanaClient {
    http: reqwest::Client,
    base_url: String,
}

impl AsanaClient {
    /// Create a new client from the `ASANA_TOKEN` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if `ASANA_TOKEN` is not set or is empty.
    pub fn from_env() -> Result<Self, Error> {
        let token = std::env::var(ENV_VAR).map_err(|_| Error::MissingToken)?;

        if token.is_empty() {
            return Err(Error::MissingToken);
        }

        Self::new(&token)
    }

    /// Create a new client with the given access token.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be initialized.
    pub fn new(token: &str) -> Result<Self, Error> {
        use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

        let mut headers = HeaderMap::new();
        let auth_value = format!("Bearer {}", token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value).map_err(|_| Error::InvalidToken)?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(Error::Http)?;

        Ok(Self {
            http,
            base_url: BASE_URL.to_string(),
        })
    }

    /// Returns the base URL for API requests.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Set a custom base URL (primarily for testing).
    #[doc(hidden)]
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }

    /// Make a GET request to the API and deserialize the response.
    ///
    /// The `path` should be the API endpoint path without the base URL (e.g., "/users/me").
    /// Query parameters can be passed via the `query` slice.
    pub async fn get<T>(&self, path: &str, query: &[(&str, &str)]) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);

        let response = self.http.get(&url).query(query).send().await?;

        self.handle_response::<DataWrapper<T>>(response)
            .await
            .map(|wrapper| wrapper.data)
    }

    /// Make a GET request expecting a list response and deserialize.
    ///
    /// Returns the first page of results along with pagination info.
    pub async fn get_list<T>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<ListWrapper<T>, Error>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);

        let response = self.http.get(&url).query(query).send().await?;

        self.handle_response::<ListWrapper<T>>(response).await
    }

    /// Make a GET request and collect all pages of results.
    ///
    /// This will automatically follow pagination until all results are collected.
    pub async fn get_all<T>(&self, path: &str, query: &[(&str, &str)]) -> Result<Vec<T>, Error>
    where
        T: DeserializeOwned,
    {
        let mut all_items = Vec::new();
        let mut offset: Option<String> = None;

        loop {
            let query_with_offset: Vec<(&str, &str)> = match &offset {
                Some(off) => {
                    let mut q = query.to_vec();
                    q.push(("offset", off.as_str()));
                    q
                }
                None => query.to_vec(),
            };

            let wrapper: ListWrapper<T> = self.get_list(path, &query_with_offset).await?;
            all_items.extend(wrapper.data);

            offset = wrapper.next_page.map(|next| next.offset);
            if offset.is_none() {
                break;
            }
        }

        Ok(all_items)
    }

    /// Make a POST request to create a resource and deserialize the response.
    ///
    /// The `path` should be the API endpoint path without the base URL.
    /// The `body` will be serialized as JSON in the request body.
    pub async fn post<T, B>(&self, path: &str, body: &B) -> Result<T, Error>
    where
        T: DeserializeOwned,
        B: serde::Serialize,
    {
        let url = format!("{}{}", self.base_url, path);

        let response = self.http.post(&url).json(body).send().await?;

        self.handle_response::<DataWrapper<T>>(response)
            .await
            .map(|wrapper| wrapper.data)
    }

    /// Make a PUT request to update a resource and deserialize the response.
    ///
    /// The `path` should be the API endpoint path without the base URL.
    /// The `body` will be serialized as JSON in the request body.
    pub async fn put<T, B>(&self, path: &str, body: &B) -> Result<T, Error>
    where
        T: DeserializeOwned,
        B: serde::Serialize,
    {
        let url = format!("{}{}", self.base_url, path);

        let response = self.http.put(&url).json(body).send().await?;

        self.handle_response::<DataWrapper<T>>(response)
            .await
            .map(|wrapper| wrapper.data)
    }

    /// Make a POST request that expects no response body (e.g., relationship operations).
    ///
    /// The `path` should be the API endpoint path without the base URL.
    /// The `body` will be serialized as JSON in the request body.
    pub async fn post_empty<B>(&self, path: &str, body: &B) -> Result<(), Error>
    where
        B: serde::Serialize,
    {
        let url = format!("{}{}", self.base_url, path);

        let response = self.http.post(&url).json(body).send().await?;

        self.handle_empty_response(response).await
    }

    /// Make a DELETE request to remove a resource or relationship.
    ///
    /// The `path` should be the API endpoint path without the base URL.
    pub async fn delete(&self, path: &str) -> Result<(), Error> {
        let url = format!("{}{}", self.base_url, path);

        let response = self.http.delete(&url).send().await?;

        self.handle_empty_response(response).await
    }

    /// Make a DELETE request with a body (for bulk operations).
    ///
    /// The `path` should be the API endpoint path without the base URL.
    /// The `body` will be serialized as JSON in the request body.
    pub async fn delete_with_body<B>(&self, path: &str, body: &B) -> Result<(), Error>
    where
        B: serde::Serialize,
    {
        let url = format!("{}{}", self.base_url, path);

        let response = self.http.delete(&url).json(body).send().await?;

        self.handle_empty_response(response).await
    }

    /// Handle an API response, converting errors as appropriate.
    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            let body = response.text().await?;
            serde_json::from_str(&body).map_err(Error::Parse)
        } else {
            Err(self.error_from_response(response).await)
        }
    }

    /// Handle an API response that should have no body.
    async fn handle_empty_response(&self, response: reqwest::Response) -> Result<(), Error> {
        let status = response.status();

        if status.is_success() {
            Ok(())
        } else {
            Err(self.error_from_response(response).await)
        }
    }

    /// Convert an error response to an Error.
    async fn error_from_response(&self, response: reqwest::Response) -> Error {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status == reqwest::StatusCode::NOT_FOUND {
            let message =
                extract_error_message(&body).unwrap_or_else(|| "resource not found".to_string());
            Error::NotFound(message)
        } else {
            let message = extract_error_message(&body).unwrap_or_else(|| {
                format!(
                    "HTTP {} {}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or("")
                )
            });
            Error::Api { message }
        }
    }
}

/// Extract the error message from an Asana API error response.
fn extract_error_message(body: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct ErrorResponse {
        errors: Vec<ErrorDetail>,
    }

    #[derive(serde::Deserialize)]
    struct ErrorDetail {
        message: String,
    }

    serde_json::from_str::<ErrorResponse>(body)
        .ok()
        .and_then(|r| r.errors.into_iter().next())
        .map(|e| e.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    /// Custom matcher that matches requests without an "offset" query parameter.
    struct NoOffset;

    impl Match for NoOffset {
        fn matches(&self, request: &Request) -> bool {
            !request.url.query_pairs().any(|(k, _)| k == "offset")
        }
    }

    #[test]
    fn test_new_client() {
        let client = AsanaClient::new("test-token").unwrap();
        assert_eq!(client.base_url(), BASE_URL);
    }

    #[test]
    fn test_empty_token_creates_client() {
        // Empty token creates a valid client; the API will reject it at request time
        let result = AsanaClient::new("");
        assert!(result.is_ok());
    }

    /// Simple test type for HTTP tests.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TestItem {
        gid: String,
        name: String,
    }

    /// Create a test client pointing at the mock server.
    fn test_client(server: &MockServer) -> AsanaClient {
        AsanaClient::new("test-token")
            .unwrap()
            .with_base_url(&server.uri())
    }

    // ========== get() tests ==========

    #[tokio::test]
    async fn test_get_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/items/123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"gid": "123", "name": "Test Item"}
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let item: TestItem = client.get("/items/123", &[]).await.unwrap();

        assert_eq!(item.gid, "123");
        assert_eq!(item.name, "Test Item");
    }

    #[tokio::test]
    async fn test_get_with_query_params() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/items"))
            .and(query_param("opt_fields", "gid,name"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"gid": "456", "name": "Queried Item"}
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let item: TestItem = client
            .get("/items", &[("opt_fields", "gid,name")])
            .await
            .unwrap();

        assert_eq!(item.gid, "456");
    }

    #[tokio::test]
    async fn test_get_404_returns_not_found() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/items/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let result: Result<TestItem, Error> = client.get("/items/missing", &[]).await;

        match &result {
            Err(Error::NotFound(msg)) => assert_eq!(msg, "resource not found"),
            _ => panic!("Expected NotFound error, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_get_404_extracts_asana_error_message() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/999"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "errors": [{"message": "project: Unknown object: 999"}]
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let result: Result<TestItem, Error> = client.get("/projects/999", &[]).await;

        match &result {
            Err(Error::NotFound(msg)) => assert_eq!(msg, "project: Unknown object: 999"),
            _ => panic!("Expected NotFound with Asana message, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_get_404_with_malformed_body_falls_back() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/999"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not json at all"))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let result: Result<TestItem, Error> = client.get("/projects/999", &[]).await;

        match &result {
            Err(Error::NotFound(msg)) => assert_eq!(msg, "resource not found"),
            _ => panic!("Expected NotFound fallback, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_get_404_with_wrong_json_structure_falls_back() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/999"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(serde_json::json!({"error": "something went wrong"})),
            )
            .mount(&server)
            .await;

        let client = test_client(&server);
        let result: Result<TestItem, Error> = client.get("/projects/999", &[]).await;

        match &result {
            Err(Error::NotFound(msg)) => assert_eq!(msg, "resource not found"),
            _ => panic!("Expected NotFound fallback, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_get_api_error_extracts_message() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/items/forbidden"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "errors": [{"message": "Not authorized"}]
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let result: Result<TestItem, Error> = client.get("/items/forbidden", &[]).await;

        match result {
            Err(Error::Api { message }) => assert_eq!(message, "Not authorized"),
            _ => panic!("Expected Api error"),
        }
    }

    #[tokio::test]
    async fn test_get_api_error_fallback_message() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/items/error"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let result: Result<TestItem, Error> = client.get("/items/error", &[]).await;

        match result {
            Err(Error::Api { message }) => assert!(message.contains("500")),
            _ => panic!("Expected Api error"),
        }
    }

    // ========== get_all() pagination tests ==========

    #[tokio::test]
    async fn test_get_all_single_page() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/items"))
            .and(NoOffset)
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {"gid": "1", "name": "Item 1"},
                    {"gid": "2", "name": "Item 2"}
                ],
                "next_page": null
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let items: Vec<TestItem> = client.get_all("/items", &[]).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].gid, "1");
        assert_eq!(items[1].gid, "2");
    }

    #[tokio::test]
    async fn test_get_all_multiple_pages() {
        let server = MockServer::start().await;

        // First page (no offset param)
        Mock::given(method("GET"))
            .and(path("/items"))
            .and(NoOffset)
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"gid": "1", "name": "Item 1"}],
                "next_page": {"offset": "page2"}
            })))
            .mount(&server)
            .await;

        // Second page
        Mock::given(method("GET"))
            .and(path("/items"))
            .and(query_param("offset", "page2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"gid": "2", "name": "Item 2"}],
                "next_page": {"offset": "page3"}
            })))
            .mount(&server)
            .await;

        // Third (final) page
        Mock::given(method("GET"))
            .and(path("/items"))
            .and(query_param("offset", "page3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"gid": "3", "name": "Item 3"}],
                "next_page": null
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let items: Vec<TestItem> = client.get_all("/items", &[]).await.unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].gid, "1");
        assert_eq!(items[1].gid, "2");
        assert_eq!(items[2].gid, "3");
    }

    #[tokio::test]
    async fn test_get_all_empty_result() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/items"))
            .and(NoOffset)
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [],
                "next_page": null
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let items: Vec<TestItem> = client.get_all("/items", &[]).await.unwrap();

        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_preserves_query_params() {
        let server = MockServer::start().await;

        // First page with query param (no offset)
        Mock::given(method("GET"))
            .and(path("/items"))
            .and(query_param("workspace", "123"))
            .and(NoOffset)
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"gid": "1", "name": "Item 1"}],
                "next_page": {"offset": "page2"}
            })))
            .mount(&server)
            .await;

        // Second page should also have workspace param
        Mock::given(method("GET"))
            .and(path("/items"))
            .and(query_param("workspace", "123"))
            .and(query_param("offset", "page2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"gid": "2", "name": "Item 2"}],
                "next_page": null
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let items: Vec<TestItem> = client
            .get_all("/items", &[("workspace", "123")])
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
    }

    // ========== post() tests ==========

    #[tokio::test]
    async fn test_post_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/items"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "data": {"gid": "new123", "name": "Created Item"}
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);

        #[derive(Serialize)]
        struct CreateRequest {
            data: CreateData,
        }
        #[derive(Serialize)]
        struct CreateData {
            name: String,
        }

        let body = CreateRequest {
            data: CreateData {
                name: "Created Item".to_string(),
            },
        };

        let item: TestItem = client.post("/items", &body).await.unwrap();

        assert_eq!(item.gid, "new123");
        assert_eq!(item.name, "Created Item");
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/items"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "errors": [{"message": "Invalid request data"}]
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let body = serde_json::json!({"data": {}});

        let result: Result<TestItem, Error> = client.post("/items", &body).await;

        match result {
            Err(Error::Api { message }) => assert_eq!(message, "Invalid request data"),
            _ => panic!("Expected Api error"),
        }
    }

    // ========== put() tests ==========

    #[tokio::test]
    async fn test_put_success() {
        let server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/items/123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"gid": "123", "name": "Updated Item"}
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let body = serde_json::json!({"data": {"name": "Updated Item"}});

        let item: TestItem = client.put("/items/123", &body).await.unwrap();

        assert_eq!(item.gid, "123");
        assert_eq!(item.name, "Updated Item");
    }

    #[tokio::test]
    async fn test_put_not_found() {
        let server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/items/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let body = serde_json::json!({"data": {}});

        let result: Result<TestItem, Error> = client.put("/items/missing", &body).await;

        assert!(matches!(result, Err(Error::NotFound(_))));
    }

    // ========== post_empty() tests ==========

    #[tokio::test]
    async fn test_post_empty_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/tasks/123/addProject"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {}
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let body = serde_json::json!({"data": {"project": "proj456"}});

        let result = client.post_empty("/tasks/123/addProject", &body).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_empty_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/tasks/123/addProject"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "errors": [{"message": "Not authorized to add to project"}]
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let body = serde_json::json!({"data": {"project": "proj456"}});

        let result = client.post_empty("/tasks/123/addProject", &body).await;

        match result {
            Err(Error::Api { message }) => assert_eq!(message, "Not authorized to add to project"),
            _ => panic!("Expected Api error"),
        }
    }

    // ========== delete() tests ==========

    #[tokio::test]
    async fn test_delete_success() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/items/123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {}
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let result = client.delete("/items/123").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/items/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let result = client.delete("/items/missing").await;

        assert!(matches!(result, Err(Error::NotFound(_))));
    }

    // ========== delete_with_body() tests ==========

    #[tokio::test]
    async fn test_delete_with_body_success() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/tasks/123/removeDependencies"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {}
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let body = serde_json::json!({"data": {"dependencies": ["dep1", "dep2"]}});

        let result = client
            .delete_with_body("/tasks/123/removeDependencies", &body)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_with_body_error() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/tasks/123/removeDependencies"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "errors": [{"message": "Invalid dependencies"}]
            })))
            .mount(&server)
            .await;

        let client = test_client(&server);
        let body = serde_json::json!({"data": {"dependencies": []}});

        let result = client
            .delete_with_body("/tasks/123/removeDependencies", &body)
            .await;

        match result {
            Err(Error::Api { message }) => assert_eq!(message, "Invalid dependencies"),
            _ => panic!("Expected Api error"),
        }
    }

    // ========== extract_error_message tests ==========

    #[test]
    fn test_extract_error_message_valid() {
        let body = r#"{"errors": [{"message": "Project not found"}]}"#;
        assert_eq!(
            extract_error_message(body),
            Some("Project not found".to_string())
        );
    }

    #[test]
    fn test_extract_error_message_empty_errors() {
        let body = r#"{"errors": []}"#;
        assert_eq!(extract_error_message(body), None);
    }

    #[test]
    fn test_extract_error_message_malformed() {
        let body = "not json";
        assert_eq!(extract_error_message(body), None);
    }

    #[test]
    fn test_extract_error_message_wrong_structure() {
        let body = r#"{"error": "Something went wrong"}"#;
        assert_eq!(extract_error_message(body), None);
    }
}
