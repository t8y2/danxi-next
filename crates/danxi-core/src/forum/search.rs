use super::{ForumService, RawFloor, SessionManager, request_error};
use crate::{AppError, CampusSession, ForumSearchPage};

const SEARCH_PAGE_SIZE: u32 = 10;

fn validate_search(query: &str, offset: u32) -> Result<&str, AppError> {
    let query = query.trim();
    if query.is_empty() || query.chars().count() > 200 {
        return Err(AppError::Validation("搜索词需为 1–200 个字符".to_owned()));
    }
    if offset.checked_add(SEARCH_PAGE_SIZE).is_none() {
        return Err(AppError::Validation("搜索位置超出范围".to_owned()));
    }
    Ok(query)
}

impl ForumService {
    pub async fn search_floors(
        &self,
        access_token: &str,
        query: &str,
        offset: u32,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<ForumSearchPage, AppError> {
        let query = validate_search(query, offset)?;
        let request = self
            .http
            .get(format!("{}/floors/search", self.forum_base))
            .bearer_auth(access_token)
            .query(&[
                ("search", query.to_owned()),
                ("offset", offset.to_string()),
                ("size", SEARCH_PAGE_SIZE.to_string()),
                ("accurate", "false".to_owned()),
            ])
            .build()?;
        let response = self.execute(request, campus, use_webvpn).await?;
        if !response.status().is_success() {
            return Err(request_error(response.status()));
        }
        let floors = response
            .json::<Vec<RawFloor>>()
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let next_offset =
            (floors.len() == SEARCH_PAGE_SIZE as usize).then_some(offset + SEARCH_PAGE_SIZE);
        Ok(ForumSearchPage {
            floors,
            offset,
            next_offset,
        })
    }
}

impl SessionManager {
    pub async fn search_floors(
        &self,
        query: &str,
        offset: u32,
        campus: Option<&CampusSession>,
        use_webvpn: bool,
    ) -> Result<ForumSearchPage, AppError> {
        let query = validate_search(query, offset)?;
        let token = self.community_token()?;
        match self
            .forum
            .search_floors(&token.access, query, offset, campus, use_webvpn)
            .await
        {
            Ok(page) => Ok(page),
            Err(AppError::Auth(_)) => {
                let refreshed = self.refresh_token(&token, campus, use_webvpn).await?;
                self.forum
                    .search_floors(&refreshed.access, query, offset, campus, use_webvpn)
                    .await
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };

    fn mock_search(status: &str, body: String) -> (ForumService, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test listener");
        let address = listener.local_addr().expect("test address");
        let status = status.to_owned();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test connection");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("test timeout");
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let length = stream.read(&mut buffer).expect("test request");
                assert!(length > 0 && request.len() < 16384);
                request.extend_from_slice(&buffer[..length]);
            }
            write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).expect("test response");
            String::from_utf8(request).expect("test request text")
        });
        let mut service = ForumService::new().expect("test service");
        service.http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("test client");
        service.forum_base = format!("http://{address}");
        (service, server)
    }

    #[tokio::test]
    async fn validates_search_before_network_access() {
        let service = ForumService::new().expect("test service");
        for (query, offset) in [
            (" ".to_owned(), 0),
            ("中".repeat(201), 0),
            ("选课".to_owned(), u32::MAX),
        ] {
            assert!(matches!(
                service
                    .search_floors("test-token", &query, offset, None, false)
                    .await,
                Err(AppError::Validation(_))
            ));
        }
        assert_eq!(validate_search("  选课  ", 0).expect("valid query"), "选课");
        assert!(validate_search(&"中".repeat(200), 0).is_ok());
    }

    #[tokio::test]
    async fn sends_encoded_query_and_returns_floor_results_with_pagination() {
        let floors = (1..=10)
            .map(|floor_id| {
                serde_json::json!({
                    "floor_id": floor_id, "hole_id": 42, "content": "选课经验", "anonyname": "同学",
                    "mention": [{ "floor_id": 100, "hole_id": 42, "content": "引用" }]
                })
            })
            .collect::<Vec<_>>();
        let (service, server) =
            mock_search("200 OK", serde_json::to_string(&floors).expect("fixture"));
        let page = service
            .search_floors("test-token", "  选课 & C++  ", 20, None, false)
            .await
            .expect("search page");
        let request = server.join().expect("test server");
        assert!(request.starts_with("GET /floors/search?"));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-token\r\n")
        );
        let target = request.split_whitespace().nth(1).expect("request target");
        let url = reqwest::Url::parse(&format!("http://localhost{target}")).expect("test URL");
        let params = url
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(params["search"], "选课 & C++");
        assert_eq!(params["offset"], "20");
        assert_eq!(params["size"], "10");
        assert_eq!(params["accurate"], "false");
        assert_eq!(page.offset, 20);
        assert_eq!(page.next_offset, Some(30));
        assert_eq!(page.floors[0].hole_id, 42);
        assert_eq!(page.floors[0].mentions[0].floor_id, 100);
        let dto = serde_json::to_value(page).expect("DTO");
        assert_eq!(dto["nextOffset"], 30);
        assert_eq!(dto["floors"][0]["floorId"], 1);
    }

    #[tokio::test]
    async fn empty_and_partial_results_end_pagination() {
        for body in ["[]", r#"[{"floor_id":1,"hole_id":42}]"#] {
            let (service, server) = mock_search("200 OK", body.to_owned());
            let page = service
                .search_floors("test-token", "选课", 0, None, false)
                .await
                .expect("search page");
            assert_eq!(page.next_offset, None);
            server.join().expect("test server");
        }
    }

    #[tokio::test]
    async fn errors_do_not_expose_query_token_or_response_body() {
        for status in ["401 Unauthorized", "500 Internal Server Error"] {
            let (service, server) = mock_search(status, "private upstream response".to_owned());
            let error = service
                .search_floors("test-token", "private-query", 0, None, false)
                .await
                .expect_err("upstream error");
            assert_eq!(
                matches!(error, AppError::Auth(_)),
                status.starts_with("401")
            );
            let message = serde_json::to_string(&error).expect("error DTO");
            for secret in ["test-token", "private-query", "private upstream response"] {
                assert!(!message.contains(secret));
            }
            server.join().expect("test server");
        }
    }
}
