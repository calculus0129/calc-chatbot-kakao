// ==========================================================================
// MessengerBot R 브리지 (TS 원본)
// ==========================================================================
//
// 빌드:  cd bot-script
//        npm install          (최초 한 번만)
//        npm run build        → dist/bridge.js 생성
//
// 사용:  MessengerBot R 에서 "새 봇" 만들고, dist/bridge.js 내용 전체 복사 →
//        봇 편집창에 붙여넣기 → 컴파일 → 전원 ON.
//
// 동작:
//   카톡 메시지 도착 → MessengerBot 이 response() 호출
//   → 우리 Rust 서버에 HTTPS POST → JSON 응답 받음
//   → replier.reply() 로 카톡 채팅창에 답장 게시
//
// 왜 TS 로 쓰는가?
//   MessengerBot 은 Rhino JS 만 돌리지만, 개발은 TS 로 해야 타입 안전 + 자동완성.
//   tsconfig.json 에서 target=ES2015 로 컴파일 → Rhino 가 먹는 형태.

// --------------------------------------------------------------------------
// 설정 — 본인 환경에 맞게 수정
// --------------------------------------------------------------------------

/**
 * Fly 배포 후 나오는 URL + `/incoming`.
 * 로컬 테스트 중이면 ngrok/cloudflared 터널 URL 로 바꿀 수 있음.
 */
const SERVER_URL = "https://calc-chatbot-kakao.fly.dev/incoming";

/**
 * 서버에서 `fly secrets set BOT_API_KEY=<값>` 으로 설정한 값과 **정확히 동일** 해야 함.
 * 로컬 테스트 중이면 BOT_API_KEY 에 쓴 값 그대로. 예: "local-dev-key".
 */
const API_KEY = "REPLACE_ME";

/**
 * HTTP 요청 timeout. Jsoup 은 **동기 호출** 이라 이 시간 동안 MessengerBot 이 멈춤.
 * 너무 길게 잡으면 폰 UI 가 버벅임. 너무 짧으면 서버 cold-start 시 실패.
 */
const TIMEOUT_MS = 5000;

// --------------------------------------------------------------------------
// Type Definitions — 서버와 계약(Contract)
// --------------------------------------------------------------------------

/**
 * 서버로 올려 보내는 본문 schema.
 * **server/src/main.rs 의 `IncomingMessage` 와 필드 이름/type 일치해야 함.**
 * 한쪽만 바꾸면 다른 쪽이 400 Bad Request 로 실패.
 */
interface OutgoingPayload {
  room: string;
  sender: string;
  msg: string;
  isGroupChat: boolean;
}

/**
 * 서버가 돌려주는 응답 스키마.
 * reply 가 없거나 null 이면 봇이 답장하지 않음 (일반 대화 무시).
 */
interface ServerReply {
  reply?: string | null;
}

// --------------------------------------------------------------------------
// Main Entrypoint — MessengerBot R 규약
// --------------------------------------------------------------------------

/**
 * 카톡 메시지 한 건이 들어올 때마다 MessengerBot 이 호출하는 callback.
 * **함수 이름/시그니처는 MessengerBot 규약** 이라 바꾸면 안 됨.
 *
 * @param room          채팅방 이름
 * @param msg           메시지 본문
 * @param sender        보낸 사람 닉네임
 * @param isGroupChat   그룹톡 여부
 * @param replier       답장 전송 객체
 */
function response(
  room: string,
  msg: string,
  sender: string,
  isGroupChat: boolean,
  replier: Replier,
): void {
  try {
    const payload: OutgoingPayload = {
      room: room,
      sender: sender,
      msg: msg,
      isGroupChat: isGroupChat,
    };

    // Rhino 는 fetch/XHR 이 없어서 Jsoup 으로 HTTP 호출.
    // Android 에 번들된 Jsoup (HTML 파서) 의 Connection API 가 fluent builder라 편하다.
    const res = org.jsoup.Jsoup.connect(SERVER_URL)
      .header("Content-Type", "application/json")
      .header("X-API-Key", API_KEY)
      .requestBody(JSON.stringify(payload))
      .ignoreContentType(true)            // JSON 허용 (Jsoup 기본은 HTML 만)
      .timeout(TIMEOUT_MS)
      .method(org.jsoup.Connection.Method.POST)
      .execute();

    const body = res.body();
    if (!body) {
      // 서버가 빈 응답 → 잠자코 종료. 에러 아닌 "no-op" 경로.
      return;
    }

    // JSON.parse 결과에 type assertion. runtime에선 그냥 any → 필요시 추가 검증.
    const json = JSON.parse(body) as ServerReply;
    if (json && json.reply) {
      replier.reply(json.reply);
    }
  } catch (e) {
    // 네트워크 장애, JSON 파싱 실패 등 모두 여기로.
    // 실서비스에선 조용히 무시하는 게 낫다 (봇이 계속 에러 답장 뿌리면 민폐).
    // 디버깅 필요 시 아래 주석 해제 → MessengerBot 앱 "로그" 탭에서 확인.
    //
    // Log.e("bridge", "error: " + e);
  }
}

// --------------------------------------------------------------------------
// MessengerBot R Lifecycle Hooks (사용 안 함, 규약상 선언만)
// --------------------------------------------------------------------------

function onStartCompile(): void {}
function onCreate(_savedInstanceState: unknown, _activity: unknown): void {}
function onResume(_activity: unknown): void {}
function onPause(_activity: unknown): void {}
function onStop(_activity: unknown): void {}
