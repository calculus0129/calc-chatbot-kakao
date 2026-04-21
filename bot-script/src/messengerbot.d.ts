// ==========================================================================
// MessengerBot R Ambient Type Declarations
// ==========================================================================
//
// MessengerBot R 의 runtime은 Node/브라우저가 아니라 **Android 위에서 돌아가는
// Rhino JS 엔진** 이다. Rhino 는 JS 에서 Java 객체를 점표기로 그대로 호출할 수 있는
// "LiveConnect" 기능을 제공해서, `org.jsoup.Jsoup.connect(url)` 같은 코드가 가능하다.
//
// 문제는 TS 가 이걸 모른다 → 컴파일 에러. 그래서 여기서 **우리가 실제로 쓰는 만큼만**
// ambient 선언으로 type을 알려준다. "실제 runtime에 존재한다" 고 컴파일러에게 약속만
// 하는 파일. 출력 JS 에는 아무것도 안 찍힌다 (d.ts 는 emit 안 됨).
//
// 필요해지는 객체가 생기면 여기에 선언 추가.

/**
 * 채팅방에 답장을 보내는 객체. MessengerBot 이 `response()` 콜백의 인자로 주입.
 * 오버로드:
 *   - reply(message)              : 지금 이 방에 보냄
 *   - reply(room, message)        : 임의의 방에 보냄 (같은 채팅앱 내)
 */
declare interface Replier {
  reply(message: string): boolean;
  reply(room: string, message: string): boolean;
}

/** Jsoup HTTP 요청의 응답 객체. 우리는 본문만 읽는다. */
declare interface JsoupResponse {
  body(): string;
  statusCode(): number;
}

/**
 * Jsoup Connection: fluent builder. 체인으로 헤더/바디/timeout 등 설정 후
 * `.execute()` 하면 실제 HTTP 호출이 발생.
 */
declare interface JsoupConnection {
  header(name: string, value: string): JsoupConnection;
  requestBody(body: string): JsoupConnection;
  /** Jsoup 은 기본적으로 HTML 만 받아들임. JSON 쓰려면 true 로 완화해야 함. */
  ignoreContentType(flag: boolean): JsoupConnection;
  timeout(ms: number): JsoupConnection;
  method(method: unknown): JsoupConnection;
  execute(): JsoupResponse;
}

/**
 * Java namespace를 JS 에서 그대로 점표기로 접근 (Rhino LiveConnect).
 * 예: `org.jsoup.Jsoup.connect("https://...")`
 */
declare const org: {
  jsoup: {
    Jsoup: {
      connect(url: string): JsoupConnection;
    };
    Connection: {
      Method: {
        POST: unknown;
        GET: unknown;
      };
    };
  };
};

/**
 * MessengerBot R 전역 로거. 앱 내 "로그" 탭에 찍힘.
 * 디버깅 시 유용: `Log.e("bridge", "something went wrong: " + e);`
 */
declare const Log: {
  d(tag: string, message?: string): void;
  e(tag: string, message?: string): void;
  i(tag: string, message?: string): void;
  w(tag: string, message?: string): void;
};
