// Manual end-to-end debug of the campus authentication chain.
// Run with real credentials:
//   CAMPUS_ID=... CAMPUS_PASSWORD=... cargo test -p danxi-core --test manual_flow -- --ignored --nocapture

use danxi_core::{CampusCredentialStore, CampusCredentials, CampusService, CampusSession};
use std::sync::Arc;

struct MemoryStore(std::sync::Mutex<Option<CampusCredentials>>);

impl CampusCredentialStore for MemoryStore {
    fn load(&self) -> Result<Option<CampusCredentials>, danxi_core::AppError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn save(&self, credentials: &CampusCredentials) -> Result<(), danxi_core::AppError> {
        *self.0.lock().unwrap() = Some(credentials.clone());
        Ok(())
    }
    fn clear(&self) -> Result<(), danxi_core::AppError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

#[tokio::test]
#[ignore = "requires real credentials"]
async fn debug_service_page() {
    let id = std::env::var("CAMPUS_ID").expect("CAMPUS_ID");
    let password = std::env::var("CAMPUS_PASSWORD").expect("CAMPUS_PASSWORD");
    let is_graduate = std::env::var("CAMPUS_GRADUATE").is_ok();

    let service = CampusService::new().unwrap();
    service.login(&id, &password).await.expect("login failed");
    println!("✓ login ok");

    let url = if is_graduate {
        "http://yjsxk.fudan.edu.cn/yjsxkapp/sys/xsxkappfudan/xsxkCourse/loadKbxx.do?_=1"
    } else {
        "https://fdjwgl.fudan.edu.cn/student/for-std/course-table"
    };

    // Reproduce service_page with visibility.
    match service.service_page(url).await {
        Ok(body) => println!(
            "✓ service_page: {} bytes, head: {}",
            body.len(),
            &body.chars().take(150).collect::<String>()
        ),
        Err(error) => println!("✗ service_page error: {error}"),
    }
}
