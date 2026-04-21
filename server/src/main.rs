// calc-chatbot-kakao — Rust 백엔드 서버
//
// 역할: MessengerBot R (안드로이드 폰) 이 카톡 메시지를 받아서
//       우리 서버의 POST /incoming 으로 JSON 을 쏴주면,
//       적절한 답장을 JSON 으로 돌려준다.
//
// 구조는 의도적으로 얇게: 라우터 2개 (/health, /incoming) + handler 함수 + 명령어 분기.

use axum::{
    extract::State,                   // handler에 공유 상태(AppState) 주입
    http::{HeaderMap, StatusCode},    // 요청 헤더 읽기 + 에러용 상태코드
    response::Json,                   // Json(...) 으로 감싸면 자동 직렬화/응답
    routing::{get, post},             // HTTP 메서드별 라우트
    Router,
};
use serde::{Deserialize, Serialize};  // JSON <-> struct 매핑. derive macro로 자동 생성.
use std::{net::SocketAddr, sync::Arc};
// Arc = Atomically Reference Counted. 여러 태스크/스레드가 같은 데이터를 공유할 때 쓰는
// smart pointer. clone() 하면 포인터만 복제되고 본체는 하나. axum handler는 요청마다
// 돌기 때문에 상태를 Arc 로 감싸서 넘기는 게 관용적 패턴.

// ------------------------------------------------------------------
// Type Definitions
// ------------------------------------------------------------------

/// 서버 전역 상태. 현재는 API 키 하나뿐이지만,
/// 나중에 DB 풀, 캐시, 설정 등 여기 추가될 자리.
#[derive(Clone)]
struct AppState {
    api_key: String,
}

/// 폰(bridge.ts) → 서버 로 올라오는 JSON 본문 schema.
/// bot-script/src/bridge.ts 의 `OutgoingPayload` 와 **모양이 일치해야** 함.
#[derive(Debug, Deserialize)]
struct IncomingMessage {
    /// 채팅방 이름 (카톡에 표시되는 그대로)
    room: String,
    /// 보낸 사람 닉네임
    sender: String,
    /// 메시지 본문 (예: "!ping", "!echo 안녕")
    msg: String,
    /// 그룹톡이면 true, 1:1 이면 false.
    /// JSON 쪽은 camelCase, Rust 쪽은 snake_case 가 관례라서 rename 으로 매핑.
    #[serde(rename = "isGroupChat")]
    is_group_chat: bool,
}

/// 서버 → 폰 응답. `reply` 가 `None` 이면 봇은 아무 말도 하지 않는다
/// (JSON 으로는 `"reply": null` 로 직렬화 됨).
#[derive(Serialize)]
struct OutgoingReply {
    reply: Option<String>,
}

// ------------------------------------------------------------------
// Entrypoint
// ------------------------------------------------------------------

// `#[tokio::main]` 은 `main` 을 비동기 runtime (tokio) 으로 감싸주는 macro.
// 실제로는 runtime을 만들고 그 위에서 async fn main 을 block_on 해주는 코드로 펼쳐진다.
#[tokio::main]
async fn main() {
    // 로그 초기화.
    // 환경변수 RUST_LOG 로 레벨 조절: `RUST_LOG=debug cargo run` 하면 debug 까지 찍힘.
    // 없으면 기본 "info".
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // API 키는 **반드시 환경변수로** 주입. 소스에 하드코딩하면 깃에 올라가서 큰일 남.
    //   로컬 PowerShell: $env:BOT_API_KEY="local-dev-key"
    //   로컬 bash:       export BOT_API_KEY=local-dev-key
    //   Fly:             fly secrets set BOT_API_KEY=<긴 랜덤 값>
    //
    // .expect(...) 는 실패 시 프로세스를 panic 으로 죽이며 메시지 출력.
    // "키가 없으면 아예 뜨지 말라" 는 뜻 — 잘못된 상태로 기동 방지.
    let api_key = std::env::var("BOT_API_KEY")
        .expect("BOT_API_KEY must be set (e.g. `fly secrets set BOT_API_KEY=...`)");

    // Arc 로 감싸는 이유:
    //   axum handler는 여러 요청을 동시에 처리할 수 있고,
    //   그때마다 AppState 를 공유해야 한다. 읽기만 하므로 Mutex 는 불필요.
    let state = Arc::new(AppState { api_key });

    // 라우터 구성. `.with_state(state)` 로 모든 handler에 state 주입.
    let app = Router::new()
        .route("/health", get(health))
        .route("/incoming", post(incoming))
        .with_state(state);

    // Fly.io 는 컨테이너에 `PORT` 환경변수로 listening port를 알려준다 (일반 관례).
    // 로컬에서는 설정 안 돼 있으므로 기본 8080.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    // `[0, 0, 0, 0]` = 모든 네트워크 인터페이스에 bind port. 컨테이너 안에서는 필수.
    // localhost(127.0.0.1) 에만 bind하면 Fly 프록시가 못 닿음.
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("listening on {}", addr);

    // TCP listener 열고 axum 에 넘김. `.await` = 이 연산이 끝날 때까지 비동기로 대기.
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ------------------------------------------------------------------
// Handlers
// ------------------------------------------------------------------

/// Health check. Fly 의 health-check 설정이 주기적으로 GET /health 때린다.
/// 200 OK 로 "ok" 만 돌려주면 성공.
/// 반환 type `&'static str` = 프로그램 생존기간 내내 유효한 문자열 슬라이스
/// (여기선 리터럴 "ok").
async fn health() -> &'static str {
    "ok"
}

/// 카톡 메시지 한 건 처리.
///
/// axum 의 마법: parameter 순서/type만으로 추출기(extractor)가 동작한다.
///   - `State(...)`  → 위에서 `.with_state(state)` 로 주입한 값
///   - `HeaderMap`   → 요청 헤더 전체 맵
///   - `Json<T>`     → 본문을 T 로 역직렬화 (실패 시 axum 이 자동 400)
///
/// 반환 `Result<Json<...>, StatusCode>`:
///   Ok  → 200 + JSON 바디
///   Err → 지정한 상태코드 (여기선 401)
async fn incoming(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(msg): Json<IncomingMessage>,
) -> Result<Json<OutgoingReply>, StatusCode> {
    // API Key Authentication.
    // 헤더 이름은 대소문자 무관하지만 내부 저장은 소문자라서 "x-api-key" 로 조회.
    let key = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())  // 헤더 값이 UTF-8 아니면 None
        .unwrap_or("");                  // 아예 없으면 빈 문자열 → 아래 비교에서 실패

    if key != state.api_key {
        // 401 Unauthorized. 잘못된 키로 계속 때리는 애 있으면 로그로 보자.
        tracing::warn!("unauthorized attempt");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Structured logging. Fly logs / 로컬 터미널에 key=value 형식으로 찍힌다.
    // `%` 는 Display trait로 찍으라는 지시 (토큰 포함하지 않도록 주의).
    tracing::info!(
        room = %msg.room,
        sender = %msg.sender,
        group = msg.is_group_chat,
        msg = %msg.msg,
        "incoming"
    );

    // 실제 처리는 순수 함수로 분리 → 테스트 쓰기 편함.
    Ok(Json(OutgoingReply {
        reply: handle(&msg.msg),
    }))
}

// ------------------------------------------------------------------
// Command Dispatcher
// ------------------------------------------------------------------

/// 텍스트 입력 한 줄을 보고 답장을 생성.
/// - `Some(s)` : 봇이 `s` 를 답한다
/// - `None`    : 봇이 조용히 있는다 (일반 대화, 알 수 없는 명령 등)
///
/// 이 함수만 늘려가면 봇 기능이 커짐. `main.rs` 의 나머지는 거의 안 건드려도 됨.
fn handle(text: &str) -> Option<String> {
    // 앞뒤 공백 제거. 사용자가 "  !ping " 쳐도 인식.
    let trimmed = text.trim();

    // 가장 단순한 완전일치 매칭.
    if trimmed == "!ping" {
        return Some("pong".to_string());
    }

    // `strip_prefix` : 접두사와 매치되면 나머지(rest) 를 Some 으로, 아니면 None.
    // `if let Some(rest) = ...` 는 Option 패턴매칭 + 언랩핑의 관용 표현.
    if let Some(rest) = trimmed.strip_prefix("!echo ") {
        return Some(rest.to_string());
    }

    // 새 명령어 추가 예시:
    //
    // if let Some(expr) = trimmed.strip_prefix("!calc ") {
    //     return Some(evaluate_expression(expr));
    // }

    None
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------
// `cargo test` 로 실행. 순수 함수 `handle` 만 검증하면 command 로직은 충분히 커버.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_returns_pong() {
        assert_eq!(handle("!ping"), Some("pong".to_string()));
    }

    #[test]
    fn ping_with_whitespace() {
        assert_eq!(handle("  !ping  "), Some("pong".to_string()));
    }

    #[test]
    fn echo_returns_rest() {
        assert_eq!(handle("!echo hello"), Some("hello".to_string()));
    }

    #[test]
    fn unknown_returns_none() {
        assert_eq!(handle("그냥 잡담"), None);
    }
}
