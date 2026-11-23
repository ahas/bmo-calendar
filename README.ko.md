[English](README.md) | **한국어** | [日本語](README.ja.md) | [简体中文](README.zh-CN.md)

# bmo-calendar

플랫폼의 날짜가 있는 일정을 하나의 서비스로 다룹니다. 캘린더와 일정은 brain 서비스에 메모리로 보관되며(brain이
원본이고 이 유닛은 그 하위입니다), 서로 분리된 두 영역, 곧 각 계정의 개인 영역과 그 회사의 조직 영역에 나뉘어
저장됩니다. 반복 일정은 발생 한 번마다 한 행으로 답합니다. 알람은 시각이 되면 receiver를 거쳐 supervisor에게 한
번 알립니다. 회사의 회의실과 그 밖의 자원을 예약하되, 한 자원의 두 예약이 겹치는 일은 없습니다. 개인의 Outlook
캘린더 하루치를 본인의 PC에서 본인의 BMO를 위해 읽습니다. 그리고 이것들을 콘솔 사이드바 항목 아래에 보여 주는 콘솔
번들을 함께 제공합니다.
`docs/design.md`는 이 서비스가 무엇인지를, `docs/api.md`는 그 경계를 기술합니다. 모든 라우트, 발행하는 이벤트,
배치하는 도구, 호출하는 대상과 설정이 여기에 정리되어 있습니다. 다른 유닛은 모두 각자의 API를 통해 접근합니다.

## 테스트

`cargo test`(testbed를 통해 실행)는 이 크레이트의 단일 하니스인 `tests/main.rs`를 실행합니다. brain, receiver,
계정 서비스, agency, 노드 서비스, Teams 페이지 서비스, Microsoft Graph의 대역(stand-in)을 루프백 포트에 띄워 두고
다음을 검증합니다. 처음 읽을 때 만들어지고 영역별로 유지되는 캘린더(`calendars.rs`), 관리자가 개인 영역을 읽되 그
안에서는 아무것도 쓰거나 만들지 않는 것(`administrator.rs`), 각 회사의 영역은 그 회사 사람과 관리자만 읽을 수 있고
다른 회사 영역에 대한 관리자의 쓰기는 거부되며, 도구 세션의 보유자가 그 소유자의 계정으로 행동하고, 회사로의 단
한 번의 이전(`companies.rs`), 자원과 예약, 겹치는 예약의 거부, 승인과 그 알림(`bookings.rs`), 두 예약이 겹치는지
여부(`overlap.rs`), 수신한 일정이 해당 회사에 보관되는 것(`schedules.rs`), 레이블과 함께 쓰고 바꾸고 취소하는
일정, id 기준의 멱등성, 문서에 적힌 대로의 거부, brain 스트림에 따른 갱신(`events.rs`), 기간 안에서 펼쳐지는 반복
일정과 한 회차만 따로 바꾸거나 취소하는 것(`repeats.rs`), 시각이 된 알람을 한 번 알리고 늦은 알람은 지연 시간과
함께 알리며 재시작을 거쳐도 두 번 알리지 않고 각각 그 계정이 닿는 방에 알리는 비트(`alarms.rs`), 실행 중인
Outlook 또는 Graph에서 본인의 노드로 읽는 개인의 Outlook 하루치, 본인 것이 아닌 노드에 대한 거부, 그리고 업무
코드 형식으로 읽는 제목(`outlook.rs`), 콘솔의 가드, bmo-ui 자체의 계약 해석으로 읽은 콘솔 화면의 매니페스트와
그 이벤트(`console.rs`), OpenAPI 문서(`openapi.rs`), 그리고 저장소 자체 검사(`repository.rs`)입니다. 도구
크레이트는 대역 캘린더 서비스를 상대로 하는 자체 하니스 `tools/calendar/tests/main.rs`를 갖습니다. 콘솔 번들의
검사는 `client/` 안에서 실행하는 `bun run check`입니다.

## 화면 점검

콘솔 번들의 화면은 콘솔이나 실제 유닛 없이 브라우저에서 구동합니다.

1. `cargo test --test main preview -- --ignored --nocapture`는 회사의 회의실, 예약, 조직 일정이 들어 있는 테스트
   대역 위에서 서비스를 띄우고, 응답하는 주소를 출력한 뒤 중지할 때까지 서비스합니다.
2. `client/preview/`에서 `PREVIEW_SERVICE=<that address> PREVIEW_KEY=<account> PREVIEW_ROLE=<member or
   administrator> bun x nuxi dev --port <port>`를 실행하면 그 위에서 화면을 Nuxt 앱으로 제공합니다. 클라이언트는
   레이어로 들어가고, 번들의 문구가 설치되며, relay 호출은 콘솔의 자격 증명과 해당 사용자의 키와 역할로 프록시됩니다.
   대역의 계정은 소유자 `1`(회사 `10`), 멤버 `2`(그 회사의 책임자), 멤버 `3`(승인을 기다리는 회의실의 관리자),
   멤버 `4`(다른 회사), `5`(소속 없음)입니다.
3. 브라우저 드라이버가 화면의 접근성 스냅샷을 읽고, 데스크톱 너비와 휴대폰 너비(360×780)에서 페이지 안의 치수를
   측정합니다.

점검에서 측정하는 내용은 콘솔 번들의 브라우저 테스트 `client/tests/browser/`(`client/package.json`의
`bmo.browser`에 선언되어 testbed가 실행)에 담겨 있습니다. 콘솔이나 서비스 없이 화면을 하나의 하니스 페이지에
모아 실제 브라우저에서 데스크톱 너비와 Galaxy S25 너비(412×915)로 열고, 대역이 플랫폼 관리자에게 하듯 서비스 대신
응답합니다.

## 구성

| 폴더 | 내용 |
|---|---|
| `src/` | 서비스와 그 명령: 라우트, 각 영역 brain의 캐시, 회사와 회사로의 이전, 자원과 예약, 알람과 비트, 알린 표시의 저장소, 개인의 Outlook 하루치와 그것을 노드에서 읽는 PowerShell 명령, 콘솔 화면의 선언, OpenAPI 문서 |
| `tools/` | 배포가 도구 호스트에 배치하는 캘린더 도구: 크레이트, 매니페스트, 입력 스키마, 매뉴얼 |
| `client/` | 콘솔 플러그인 번들(독립된 프로젝트) |
| `deploy/` | 서비스의 사용자 유닛 |
| `docs/` | 서비스가 무엇인지와 그 경계에 대한 정식 기술, 그리고 OpenAPI 문서 |
| `tests/` | 크레이트의 단일 테스트 하니스와 그것이 사용하는 대역 |

루트에는 Cargo 매니페스트, `Cargo.lock`, `testbed.toml`, `install.toml`, `rustfmt.toml`, `settings.toml`(바이너리에 내장되는
기본값), `CLAUDE.md`, `PERSONAL-DATA.md`, `LICENSE`가 있습니다.

## 명령

- `bmo-calendar serve`는 brain을 읽고, 1분마다 비트를 돌리며, 중지할 때까지 라우트에 응답합니다.
- `bmo-calendar health`는 설정에 지정된 서비스가 준비 상태로 응답하면 0으로 끝납니다.
- `bmo-calendar openapi`는 OpenAPI 문서를 출력합니다. 같은 내용이 `docs/openapi.json`에 있습니다.
- `bmo-calendar tool-env`는 도구가 서비스를 찾는 데이터 루트의 `env/calendar.env`를 씁니다.

설정은 바이너리에 내장된 `settings.toml`, 그다음 릴리스 옆에 있는 배포의 `settings.toml`과 `settings.local.toml`
(데이터 루트의 `env/calendar.toml`), 마지막으로 환경 변수 `BMO_CALENDAR_<TABLE>__<KEY>` 순서로 읽습니다
(`docs/api.md`, 「Settings」).
