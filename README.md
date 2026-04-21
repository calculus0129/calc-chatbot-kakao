# calc-chatbot-kakao

This project is for making my own custom chatbot for my convenience

## Goals

* Make my custom chatbot
* Hook up to the Kakaotalk messanger application
* Add many fun and usefule commands and functions

## Getting Started

<!-- These instructions will get you a copy of the project up and running on your local machine for development and testing purposes. See deployment for notes on how to deploy the project on a live system. -->

### Prerequisites

<!-- What things you need to install the software and how to install them

```
Give examples
``` -->

### Installing

<!-- A step by step series of examples that tell you how to get a development env running

Say what the step will be

```
Give the example
```

And repeat

```
until finished
```

End with an example of getting some data out of the system or using it for a little demo -->

<!-- ## Running the tests

Explain how to run the automated tests for this system

### Break down into end to end tests

Explain what these tests test and why

```
Give an example
```

### And coding style tests

Explain what these tests test and why

```
Give an example
```

## Deployment

Add additional notes about how to deploy this on a live system

## Built With

* [Dropwizard](http://www.dropwizard.io/1.0.2/docs/) - The web framework used
* [Maven](https://maven.apache.org/) - Dependency Management
* [ROME](https://rometools.github.io/rome/) - Used to generate RSS Feeds

## Contributing

Please read [CONTRIBUTING.md](https://gist.github.com/PurpleBooth/b24679402957c63ec426) for details on our code of conduct, and the process for submitting pull requests to us.

## Versioning

We use [SemVer](http://semver.org/) for versioning. For the versions available, see the [tags on this repository](https://github.com/your/project/tags). 

## Authors

* **Billie Thompson** - *Initial work* - [PurpleBooth](https://github.com/PurpleBooth)

See also the list of [contributors](https://github.com/your/project/contributors) who participated in this project.

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details -->

<!-- ## Reference

* https://gist.github.com/stephenparish/9941e89d80e2bc58a153
*  -->

---

## Project Layout

```
calc-chatbot-kakao/
├── server/          # Rust (axum) HTTP 서버 — Fly.io에 배포
│   ├── Cargo.toml
│   ├── src/main.rs
│   ├── Dockerfile
│   └── fly.toml
└── bot-script/      # MessengerBot R 에 붙여넣을 JS 브리지
    └── bridge.ts
```

데이터 흐름:
```
카톡 알림 → 안드로이드 폰의 MessengerBot R
         → HTTPS POST → Rust 서버 (Fly.io)
         → JSON 응답 → replier.reply()
         → 카톡 채팅창에 답장
```

## Dev Setup

사전 준비:
- Rust toolchain (`rustup`) — https://rustup.rs
- Docker Desktop (Optional — Fly could use remote builder)
- Fly CLI:
    PowerShell 에서 `iwr https://fly.io/install.ps1 -useb | iex`
    Bash에서 `curl -L https://fly.io/install.sh | sh`

    refer: https://fly.io/docs/flyctl/install/

- Android: KakaoTalk + MessengerBot R 설치, 알림 접근 권한 허용

### 1. 로컬 테스트

```bash
cd server
BOT_API_KEY=local-dev-key cargo run
```

For Windows Powershell,

```powershell
cd server
$env:BOT_API_KEY = "local-dev-key"
cargo run
```

다른 터미널에서:
```bash
curl http://localhost:8080/health
# => ok

curl -X POST http://localhost:8080/incoming `
  -H "Content-Type: application/json" `
  -H "X-API-Key: local-dev-key" `
  -d '{"room":"test","sender":"me","msg":"!ping","isGroupChat":false}'
# => {"reply":"pong"}

curl -X POST http://localhost:8080/incoming \
  -H "Content-Type: application/json" \
  -H "X-API-Key: wrong" \
  -d '{"room":"test","sender":"me","msg":"!ping","isGroupChat":false}'

# => (empty response, HTTP 401)

# echo test:
curl -X POST http://localhost:8080/incoming \
  -H "Content-Type: application/json" \
  -H "X-API-Key: local-dev-key" \
  -d '{"room":"test","sender":"me","msg":"!echo 안녕","isGroupChat":false}'

# => {"reply":"안녕"}

```

### 2. Fly.io 배포

```bash
cd server
fly auth login                # 이미 로그인 돼 있으면 스킵
fly launch --no-deploy        # fly.toml 이미 있으므로 기존 파일 유지 선택
                              # 앱 이름 중복이면 새 이름 입력
fly secrets set BOT_API_KEY=<적당히-긴-랜덤-문자열>
fly deploy
fly logs                      # 실시간 로그
```

배포 후 `https://calc-chatbot-kakao.fly.dev/health` 가 `ok` 를 리턴하면 성공.

`BOT_API_KEY` 값은 어딘가 안전하게 저장. `fly secrets list` 로는 값을 다시 못 봄 — 잊으면 새로 set 하면 됨.

### 3. Android (MessengerBot R)

먼저 TS → JS 빌드:
```bash
cd bot-script
npm install          # 최초 1회
npm run build        # dist/bridge.js 생성
```

봇 설정:
1. MessengerBot R → 새 봇 생성
2. [bot-script/src/bridge.ts](bot-script/src/bridge.ts) 의 상단 `SERVER_URL` / `API_KEY` 를 본인 값으로 수정 후 다시 `npm run build`
3. `bot-script/dist/bridge.js` 내용 전체를 MessengerBot 편집창에 붙여넣기
4. 저장 → 컴파일 → 전원 ON
5. 테스트 채팅방에서 `!ping` → `pong` 확인

### 4. 디버깅

- 서버 로그: `fly logs`
- 봇 로그: MessengerBot R 앱 내 로그 탭 (`bridge.js` 의 `Log.e(...)` 주석 해제)
- 401 Unauthorized: API 키 불일치
- 요청이 전혀 안 옴: 폰 알림 권한 / MessengerBot 전원 / 카톡 알림 설정 확인

### 5. 명령어 확장

[server/src/main.rs](server/src/main.rs) 의 `handle()` 함수에 분기 추가 후 `fly deploy`.

## Roadmap

### Done
- [x] Rust (axum) 서버 로컬 동작 — `/health`, `/incoming` (echo/ping 명령)
- [x] API Key Authentication + structured logging + unit tests
- [x] Dockerfile + fly.toml → Fly.io 배포 파이프라인 확립
      (`fly deploy` 한 방, health check 통과, `fly logs` 로 원격 관측)

### Next — End-to-end 연결
- [ ] TS bridge (`bot-script/src/bridge.ts`) → MessengerBot R 에 탑재 → 실제 카톡 채팅방에서 `!ping` → `pong` 왕복 확인

### Later — Features
- [ ] `!calc` 계산기 명령 (수식 parser)
- [ ] 상태 저장 (SQLite or Postgres) — 사용자별 설정, 명령 history 등
- [ ] 추가 명령 확장 (날씨, 알람, reminder 등)

### Stretch
- [ ] React 대시보드 — 메시지 log, 명령 통계, 봇 on/off 토글
- [ ] GCP Cloud Run 으로 이전 — 같은 Docker image 재사용
- [ ] CI/CD — GitHub Actions 에서 `fly deploy` 자동화

## Acknowledgments

* README.md layout by PurpleBooth (https://gist.github.com/PurpleBooth/109311bb0361f32d87a2)