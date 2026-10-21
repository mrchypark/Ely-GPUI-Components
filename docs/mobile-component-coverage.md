# 모바일 컴포넌트 지원·수용 기준 전수표

최초 작성: 2026-10-17. 증거 갱신: 2026-10-18. 공개 모듈의 export 및 gallery 소스 목록을 정적으로 대조한 inventory다. 모든 행의 수용 기준은 해당 행의 **모든 이름**에 적용한다. 데이터 타입·함수도 export 누락 방지를 위해 포함했으며, 이들을 모두 시각적 컴포넌트로 세지 않는다. cfg 조건은 원본 모듈이 기준이다. API 존재는 모바일 동작 PASS가 아니다. 이번 문서 통합에서는 다른 담당자의 실행 기록을 확인했으며 코드 수정이나 빌드, simulator/device 실행은 하지 않았다.

## 소스와 두 라인의 차이

이 문서는 **app-compatible Git GPUI 1a28cff 라인**의 export를 열거한다. registry 라인과 독립된 목록이다. `Menu::on_open_change`가 이 라인에 존재한다. 로컬 checkout의 `forms::TextInputOwner`와 `TextInput::input_owner_observer`는 다른 담당자가 소유한 미공개 IME 통합 변경이며, 이번 공개 후보의 API에 포함되지 않는다. 아래 text export 행은 이 제외를 반영한다. 메뉴는 일반 항목·즉시 hover·기존 오른쪽 열기 방식이다. `init`의 Tab 키 바인딩은 context None이다. forms/owner/IME 소스는 여기서 수정하지 않는다.

최초 inventory 관찰 HEAD: `27fcea58d5ea0a8adb5994f4cdef551a0bf94e56`. 과거 host 검증 snapshot의 core overlay 커밋은 이 라인의 `09702cf`이며, 두 라인의 실행 범위는 아래에 구분한다. Gallery의 추가 소스 변경과 문서는 core 실행 증거와 별도로 관리한다. 이번 문서 변경은 현재 native 빌드 snapshot에 포함되지 않는다.

기준: [src/lib.rs](../src/lib.rs), 각 `src/<category>/mod.rs`, [gallery registry ALL](../examples/gallery/pages/mod.rs). Gallery의 `data`는 공개 모듈 `data_display`에 대응한다. 아래 API 목록은 이 checkout에서 추출했다. `motion::curve::*` 및 공개 typography 하위 모듈은 별도 펼침 목록으로 포함한다. 연관 builder 메서드 전체의 API reference를 대체하지는 않는다.

## 증거의 경계

- E-MENU-A: 앱 메뉴 host 42 PASS; Android APK `0098ddda661c0050794bba25efd6a099f7cac83dda659942d3d53dffc9ac5341`에서 submenu 겹침 swipe, 배경 고정, Last item 30/count1, Home/resume, outside dismiss, 후속 canvas 입력 PASS. 모든 메뉴 종류·모든 상태 PASS가 아니다.
- E-MENU-U: registry 메뉴 host 60 PASS. registry native 실행 증거가 아니다.
- E-LAYOUT: 이전 각 라인의 layout 6 PASS 기록은 해당 시점의 Sheet/geometry 검증이다. 모든 layout 컴포넌트에 전이하지 않는다.
- E-CANVAS: 위 APK의 메뉴 닫힘 후 canvas drag 대조 PASS. 모든 편집 도구·pinch·maps PASS가 아니다.
- 정확한 이전 실행 범위: task-local `outputs/ely-menu-device-20261016/{result,evidence-review}.md`. 13번 단발 관찰은 원인 미규명으로 보존한다. repo의 [mobile-support](mobile-support.md)에도 요약이 있다.
- E-CORE-HOST (2026-10-18): app `7e146ac`의 forms/lists/navigation은 각각 144/39/35 PASS, registry `c8fe0a5`는 147/42/35 PASS다. 여섯 실행은 모두 exit 0이며, 테스트 중 소스 해시와 dependency pin이 유지됐다. 이는 필터별 host 테스트 수이며 컴포넌트 수나 category 전체의 native PASS가 아니다.
- E-OVERLAY-HOST (2026-10-18): 최신 core overlay 커밋은 app `09702cf`, registry `8563bc6`다. buttons/overlays/menus는 app 12/39/47 PASS, registry 20/39/65 PASS이며 여섯 실행 모두 exit 0이다. Popover/HoverCard의 bounds·scroll·touch disclosure 및 공간 부족 시 panel을 만들거나 focus를 가져가지 않는 회귀 범위를 포함한다. 앞선 E-CORE-HOST는 별도 snapshot의 기록이며 이번 문서 갱신에서 재실행하지 않았다.
- E-GALLERY-HOST: 기록된 retry snapshot에서 app과 upstream gallery의 host 테스트가 각각 10개 PASS했다. 각 navigation 테스트는 등록된 44개 페이지를 모두 390×844 logical pixels에서 그린다. catalog/assets, header를 고정한 catalog·specimen wheel scroll, terminal replay 4개, fixture cleanup도 검사한다. 양쪽 cleanup 로그는 수동 삭제 없이 fixture가 자동 제거됐음을 확인한다. 이는 각 페이지의 모든 demo·상태·터치 동작이나 실제 PTY/shell을 검증한 결과가 아니다.
- E-NATIVE-LIMITED: 초기 바이너리의 catalog→buttons tap/즉시 render와 후속 trace 바이너리의 terminal tap 관측을 구분한다. 후자는 gallery `f98d000` 내용이며 새 propagation/tooltip/dialog/MenuBar/table 코드를 포함하지 않는다. ANSI replay, selector reload/menu와 zsh 선택, Clear, Stop의 no-process 안내, 검색 previous, tab 왕복 시 상태 유지, 세로 split과 기존 빈 pane 유지만 관측했다. 로컬 PTY/shell·IME·모든 terminal 동작의 PASS가 아니다. Trace drag는 Started/Ended만 수신하고 accepted Moved가 없었으므로 정상 finger scroll 수용 증거도 아니다. 근거는 task-local `outputs/mobile-component-progress-20261017.md`의 native trace runtime 및 terminal native tap evidence 항목이다.
- 현재 증거 경로는 task-local `outputs/mobile-gallery-simulator-20261017/provenance/`다. 이전 core는 `host-owned-fixed-20261018-101757-32699-results.txt`, 같은 prefix의 여섯 suite 로그와 `-integrity.txt`를 참조한다. 최신 overlay는 `frozen-batch-20261018-104222-48546-results.txt`와 여섯 suite 로그를 참조한다. Gallery는 `gallery-retry-20261018-105023-50471`의 양쪽 runtime/results/cleanup 로그를 참조하며, 이 재실행 결과가 이전 frozen-batch의 gallery 실패 기록을 대체한다. 저장소에 이 산출물이 포함된다는 뜻은 아니다.
- E-SOURCE-FROZEN: compound wrapper의 창별 bounds 전달, gallery의 명시적 bounds 인자, TooltipTrigger/HelpTooltip 및 FormLabel/FormField, Dialog wrapper와 no-room 보정, MenuBar bounds, DataGrid/Spreadsheet touch 조작은 소스 구현·동결 단계다. 새 테스트는 작성됐으며 진행 중인 배치의 compile/runtime 성공을 이 문서에서 선언하지 않는다. 앞선 E-CORE-HOST/E-OVERLAY-HOST/E-GALLERY-HOST 수치는 명시된 과거 snapshot의 결과다.
- 최신 소스 근거는 task-local `outputs/mobile-compound-overlay-handoff-20261018.md`, `mobile-wrapper-interaction-tests-handoff-20261018.md`, `tooltip-form-bounds-freeze-20261018.md`, `dialogs-usable-bounds-20261018.md`, `gallery-overlay-bounds-handoff-20261018.md`, `gallery-menubar-bounds-handoff-20261018.md`, `tables-touch-freeze-20261018.md`다(모두 outputs 아래). 후속 MenuBar handoff가 이전 gallery handoff의 setter 미구현 설명을 대체한다. Theme/global geometry는 추가하지 않았으며 native host 전달과 새 코드의 실행 수용은 별도 검증으로 남는다.
- 아래 category는 위의 좁은 실행 범위를 제외하면 컴포넌트별 simulator/device 수용 증거가 없다. 소스 테스트 anchor 자체는 PASS가 아니며, native IME, rotation, multitouch, GPS/BLE, 접근성, 성능 또는 iOS 전체 지원을 주장하지 않는다.

## 공통 simulator 수용 시나리오

각 시각적 export를 gallery에서 하나씩 실행하고 결과를 이름별로 기록한다. 데이터/함수 export는 그것을 소비하는 컴포넌트와 순수 테스트로 검증한다. 한 대표 컴포넌트 PASS로 같은 category를 일괄 PASS 처리하지 않는다.

- S0: 280/390 logical-width phone 및 wide tablet, portrait/landscape, notch/home inset; 큰 글자·긴 번역·RTL·light/dark에서 잘림/겹침과 읽기 순서 확인.
- S1: tap 한 번당 action 한 번, disabled 무반응, pressed 해제, drag 밖 release/cancel, 작은 icon hit 영역; hover 없이 모든 필수 작업 접근.
- S2: 짧고 긴 content, 양 끝 overscroll, 부모/자식 및 overlay 겹침 swipe; 대상만 이동. 닫힌 overlay 뒤 정상 scroll 대조 필수.
- S3: keyboard show/hide와 focus reveal, selection/caret, 입력 후 overlay focus 반환; 실제 IME 조합은 backend 담당과 별도 실기기 검증.
- S4: modal/Sheet/menu open, 내부 navigation, outside/Back/Escape 닫기, Home/resume 및 surface 재생성. 숨은 overlay가 터치를 잡거나 열린 overlay에서 canvas로 누출되면 실패.
- S5: drag/pinch 시작·중간·취소·추가 손가락·회전, 한 gesture 한 owner; model과 실제 화면을 함께 비교. simulator 마우스만으로 multitouch PASS 금지.
- S6: OS 서비스는 성공/거부/취소/미지원 응답으로 UI 검증. simulator에 없는 camera/mic/filesystem/PTY/native view 기능은 명시적 제한 또는 대체 UI를 요구하고 실기기 항목으로 남긴다.
- S7: VoiceOver/TalkBack label·role·focus order·동작, reduced motion, 대량 데이터 스크롤 및 백그라운드 복귀. FPS 목표는 아직 정의하지 않았으므로 성능 PASS 선언 금지.

## 다음 구현 우선순위

1. Buttons와 일부 forms/lists/navigation에 연결된 touch 정책 및 host 회귀 결과를 기준으로, 아직 적용하지 않은 export를 개별 점검하고 S1/S7의 native 동작을 검증한다. 테마 숫자나 일부 host PASS를 전체 완료로 보지 않는다.
2. 기존 ActionSheet/Popover/HoverCard/PieMenu와 새 compound/Tooltip/MenuBar 연결을 실제 호출부에서 검증한다. 소스 구현과 새 테스트 작성은 native 수용 완료가 아니다. Native Tooltip body factory는 소비자의 touch disclosure를 자동 생성하지 않는다.
3. 기존 `Tabs`, `NavItem`, `Sidebar`, `MasterDetail`을 compact 배치로 조합한다. 별도 BottomNavigation API는 현재 계획이며 기존 export로 오인하지 않는다.
4. forms/editor/documents와 목록의 선택·키보드·scroll arbitration을 검증한다. `SwipeableListItem`의 기존 two-finger swipe 설명은 한 손가락 native 지원 보장이 아니다.
5. desktop shell/PTY/drag-resize 기능은 아래 대체 UX로 분리한다. 실제 OS 서비스 연결을 Ely 내부에 추가하지 않는다.

## 카테고리별 실제 export와 수용 기준

아래 source 그룹 행은 해당 그룹의 모든 이름을 빠짐없이 적는다. 각 category의 수용 시나리오는 그 아래 모든 행에 적용된다. E-MENU/E-LAYOUT/E-CORE-HOST/E-OVERLAY-HOST/E-GALLERY-HOST/E-NATIVE-LIMITED는 각각 명시한 범위에만 해당하며 category 전체로 확대하지 않는다. 44개 gallery 페이지, export 선언 그룹, 개별 시각적 컴포넌트는 서로 다른 집계 단위다.

### account

공개 경계: [src/account/mod.rs](../src/account/mod.rs).

상태: 입력/선택 UI 존재, native IME 및 각 compound form 검증 없음. S0/S1/S2/S3/S4/S7. keyboard-only shortcut/scrub에 버튼 또는 직접 입력 대안; 업로드·인증·전송은 host 서비스이며 fixture action만 사용.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use billing::{ BillingHistory, Invoice, InvoiceState, Quota, Standing, Subscription, SubscriptionCard, UsageQuota, quota_line, };` | `mod.rs:18` |
| `pub use factor::TwoFactorInput;` | `mod.rs:22` |
| `pub use keys::{ApiKey, ApiKeyManager};` | `mod.rs:23` |
| `pub use login::{Login, LoginForm};` | `mod.rs:24` |
| `pub use mail::{ForgotPassword, MagicLinkForm};` | `mod.rs:25` |
| `pub use oauth::OAuthButtons;` | `mod.rs:26` |
| `pub use payment::{CardBrand, CardDetails, ExpiryError, PaymentMethodForm, expiry, luhn};` | `mod.rs:27` |
| `pub use profile::{Profile, ProfileCard, ProfileEditor};` | `mod.rs:28` |
| `pub use sessions::{Session, SessionList};` | `mod.rs:29` |
| `pub use signup::{Signup, SignupForm, password_rules};` | `mod.rs:30` |
| `pub use switcher::{Account, AccountSwitcher, UserMenu, Workspace, WorkspaceSwitcher};` | `mod.rs:31` |
| `pub use team::{Invitation, InvitationList, Member, Role, RoleSelector, TeamMemberTable};` | `mod.rs:32` |
| `pub use upgrade::UpgradePrompt;` | `mod.rs:33` |

소스 테스트 anchor (실행 상태 아님): `src/account/tests/sign_in.rs src/account/tests/mod.rs src/account/tests/billing.rs src/account/tests/access.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/account/access.rs`
- `examples/gallery/pages/account/billing.rs`
- `examples/gallery/pages/account/mod.rs`
- `examples/gallery/pages/account/sign_in.rs`

### agent

공개 경계: [src/agent/mod.rs](../src/agent/mod.rs).

상태: 복합 화면·모델 API 존재, mobile 전수 증거 없음. S0/S1/S2/S3/S4/S6/S7. composer와 keyboard reveal, streaming 중 scroll 유지, long action rows, media 권한/취소 확인; AI/network/call 서비스는 host fixture.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use artifact::ArtifactPanel;` | `mod.rs:15` |
| `pub use changes::{ChangeState, FileChange, FileChangeCard, MultiFileDiffReview};` | `mod.rs:16` |
| `pub use control::{CostBreakdown, HumanInputRequest};` | `mod.rs:17` |
| `pub use environment::{McpServer, McpServerList, SandboxState, SandboxStatus, ServerState};` | `mod.rs:18` |
| `pub use memory::{Checkpoint, CheckpointList, Memory, MemoryPanel};` | `mod.rs:19` |
| `pub use permission::{Permission, PermissionPrompt, ToolApprovalDialog};` | `mod.rs:20` |
| `pub use previews::{BrowserPreview, ComputerUseViewer, LivePreview};` | `mod.rs:21` |
| `pub use progress::{AgentPlan, AgentProgress, AgentState, AgentStatus, AgentStep, AgentStepList};` | `mod.rs:22` |
| `pub use registry::{RegisteredTool, ToolAccess, ToolRegistry};` | `mod.rs:23` |
| `pub use tools::{ToolCallCard, ToolCallGroup};` | `mod.rs:24` |

소스 테스트 anchor (실행 상태 아님): `src/agent/tests/environment.rs src/agent/tests/mod.rs src/agent/tests/changes.rs src/agent/tests/control.rs src/agent/tests/previews.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/agent/changes.rs`
- `examples/gallery/pages/agent/control.rs`
- `examples/gallery/pages/agent/environment.rs`
- `examples/gallery/pages/agent/mod.rs`
- `examples/gallery/pages/agent/previews.rs`
- `examples/gallery/pages/agent/progress.rs`
- `examples/gallery/pages/agent/tools.rs`

### buttons

공개 경계: [src/buttons/mod.rs](../src/buttons/mod.rs).

상태: 기존 UI와 테마 API 존재; 모바일 hit 영역·접근성 전수 검증 없음. S0/S1/S4/S7. ActionSheet 기존 구현 재사용, Tooltip에 필수 action을 숨기지 않고 visible label/설명으로 대체. Theme/enum은 소비 위젯에서 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use bars::{ActionBar, BulkActionBar};` | `mod.rs:20` |
| `pub use button::{Button, ButtonVariant};` | `mod.rs:22` |
| `pub use confirm::{ConfirmButton, ConfirmMode};` | `mod.rs:23` |
| `pub use copy::CopyButton;` | `mod.rs:24` |
| `pub use fab::FloatingActionButton;` | `mod.rs:25` |
| `pub use group::{ButtonGroup, ToggleButton, ToggleGroup, ToggleItem};` | `mod.rs:26` |
| `pub use icon_button::IconButton;` | `mod.rs:27` |
| `pub use presets::{back_button, close_button, more_button};` | `mod.rs:28` |
| `pub use quick::{QuickAction, QuickActions};` | `mod.rs:29` |
| `pub use segmented::SegmentedControl;` | `mod.rs:30` |
| `pub use share::ShareButton;` | `mod.rs:31` |
| `pub use sheet::ActionSheet;` | `mod.rs:32` |

소스 테스트 anchor (실행 상태 아님): `src/buttons/tests.rs src/buttons/touch_tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/buttons/actions.rs`
- `examples/gallery/pages/buttons/basics.rs`
- `examples/gallery/pages/buttons/choices.rs`
- `examples/gallery/pages/buttons/mod.rs`

### calendar

공개 경계: [src/calendar/mod.rs](../src/calendar/mod.rs).

상태: 공개 API 존재, mobile 실행 증거 미확인. S0–S7 중 UI가 제공하는 동작을 모두 검사하고 비적용 사유를 명시.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use agenda::AgendaView;` | `mod.rs:25` |
| `pub use allday::AllDayRow;` | `mod.rs:26` |
| `pub use availability::AvailabilityPicker;` | `mod.rs:27` |
| `pub use card::EventCard;` | `mod.rs:28` |
| `pub use chip::EventChip;` | `mod.rs:29` |
| `pub use days::{CalendarDayView, CalendarDays, CalendarWeekView};` | `mod.rs:30` |
| `pub use editor::{EventDraft, EventEditor};` | `mod.rs:31` |
| `pub use event::{Event, When};` | `mod.rs:32` |
| `pub use grid::{TimeGrid, TimezoneOverlay};` | `mod.rs:33` |
| `pub use month::CalendarMonthView;` | `mod.rs:34` |
| `pub use now::CurrentTimeIndicator;` | `mod.rs:35` |
| `pub use people::{Answer, Attendee, AttendeeList};` | `mod.rs:36` |
| `pub use popover::EventPopover;` | `mod.rs:37` |
| `pub use recurrence::RecurrenceEditor;` | `mod.rs:38` |
| `pub use reminders::ReminderPicker;` | `mod.rs:39` |
| `pub use repeat::{Ends, Frequency, Recurrence};` | `mod.rs:40` |
| `pub use year::YearView;` | `mod.rs:41` |

소스 테스트 anchor (실행 상태 아님): `src/calendar/tests/people.rs src/calendar/tests/mod.rs src/calendar/tests/editing.rs src/calendar/tests/times.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/calendar/editing.rs`
- `examples/gallery/pages/calendar/mod.rs`
- `examples/gallery/pages/calendar/months.rs`
- `examples/gallery/pages/calendar/people.rs`
- `examples/gallery/pages/calendar/times.rs`

### canvas

공개 경계: [src/canvas/mod.rs](../src/canvas/mod.rs).

상태: gesture/geometry API 존재; E-CANVAS의 좁은 drag 외 도구별 native 증거 없음. S0/S1/S2/S5/S7. resize/rotate/selection handles는 touch hit 영역과 cancel 복구 확인; mouse-wheel-only 조작은 +/- 및 reset 대안. 지도 GPS는 별도 서비스.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use align::{Align, AlignmentToolbar, aligned};` | `mod.rs:38` |
| `pub use assets::AssetPanel;` | `mod.rs:39` |
| `pub use autolayout::{AutoLayout, AutoLayoutControls, Flow, Place};` | `mod.rs:40` |
| `pub use constraints::{Constraints, ConstraintsEditor, Pin};` | `mod.rs:41` |
| `pub use edit::{Handle, Tool};` | `mod.rs:42` |
| `pub use export::{ExportFormat, ExportPanel, ExportSetting, export_files};` | `mod.rs:43` |
| `pub use flow::{FlowChart, Step, StepKind};` | `mod.rs:44` |
| `pub use gesture::Brush;` | `mod.rs:45` |
| `pub use graph::{Edge, Node, Port};` | `mod.rs:46` |
| `pub use history::HistoryPanel;` | `mod.rs:47` |
| `pub use inspector::InspectorPanel;` | `mod.rs:48` |
| `pub use layers::LayerPanel;` | `mod.rs:49` |
| `pub use links::Link;` | `mod.rs:50` |
| `pub use marks::{SelectionBox, SnapIndicator, TransformHandles};` | `mod.rs:51` |
| `pub use mindmap::Topic;` | `mod.rs:52` |
| `pub use mindmap_view::MindMap;` | `mod.rs:53` |
| `pub use minimap::MiniMap;` | `mod.rs:54` |
| `pub use nodes::NodeGraph;` | `mod.rs:55` |
| `pub use palette::{BrushSettings, ToolPalette};` | `mod.rs:57` |
| `pub use plane::InfiniteCanvas;` | `mod.rs:59` |
| `pub use shape::{Artboard, Corner, Guide, Shape, ShapeKind};` | `mod.rs:60` |
| `pub use style::{Border, BorderEditor, ColorPanel, Dash, Shadow, ShadowEditor};` | `mod.rs:61` |
| `pub use tools::ToolLayer;` | `mod.rs:62` |
| `pub use touch::{CanvasTouch, TouchAction, TouchContact};` | `mod.rs:63` |
| `pub use typography::{TypeStyle, TypographyPanel};` | `mod.rs:64` |
| `pub use view::{Frame, Viewport};` | `mod.rs:65` |
| `pub use whiteboard::{Board, Whiteboard};` | `mod.rs:66` |
| `pub use zoom::{ZoomControls, ZoomIndicator};` | `mod.rs:67` |

소스 테스트 anchor (실행 상태 아님): `src/canvas/tests/panels.rs src/canvas/tests/mod.rs src/canvas/tests/boards.rs src/canvas/tests/stale.rs src/canvas/tests/pinch.rs src/canvas/tests/tools.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/canvas/boards.rs`
- `examples/gallery/pages/canvas/graphs.rs`
- `examples/gallery/pages/canvas/looks.rs`
- `examples/gallery/pages/canvas/mod.rs`
- `examples/gallery/pages/canvas/panels.rs`
- `examples/gallery/pages/canvas/plane.rs`
- `examples/gallery/pages/canvas/tools.rs`

### charts

공개 경계: [src/charts/mod.rs](../src/charts/mod.rs).

상태: 데이터 표시 UI 존재, mobile chart 탐색·dense dashboard 검증 없음. S0/S1/S2/S5/S7. hover tooltip은 tap/선택 detail 대체, 범례·축·범위 선택 접근성 확인; 실제 금융 주문/서비스 실행 없이 fixture callback 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use burndown::BurndownChart;` | `mod.rs:32` |
| `pub use calendar::CalendarHeatmap;` | `mod.rs:33` |
| `pub use cartesian::{AreaChart, BarChart, LineChart, ScatterChart};` | `mod.rs:34` |
| `pub use distribution::{BoxPlot, Histogram, ViolinPlot, WaterfallChart};` | `mod.rs:35` |
| `pub use flow::SankeyChart;` | `mod.rs:36` |
| `pub use funnel::FunnelChart;` | `mod.rs:37` |
| `pub use network::NetworkGraph;` | `mod.rs:38` |
| `pub use parallel::ParallelCoordinates;` | `mod.rs:39` |
| `pub use parts::{ChartLegend, ChartTooltip};` | `mod.rs:40` |
| `pub use pie::PieChart;` | `mod.rs:41` |
| `pub use radar::RadarChart;` | `mod.rs:42` |
| `pub use realtime::RealtimeChart;` | `mod.rs:43` |
| `pub use relations::ChordDiagram;` | `mod.rs:44` |
| `pub use schedule::{GanttChart, Task};` | `mod.rs:45` |
| `pub use series::LIMIT;` | `mod.rs:46` |
| `pub use series::{Points, Series};` | `mod.rs:47` |
| `pub use sunburst::{Slice, Sunburst};` | `mod.rs:48` |
| `pub use targets::{Bullet, BulletChart, ProgressChart};` | `mod.rs:49` |
| `pub use tiles::{HeatmapChart, Treemap};` | `mod.rs:50` |

소스 테스트 anchor (실행 상태 아님): `src/charts/tests/mod.rs src/charts/tests/limits.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/charts/cartesian.rs`
- `examples/gallery/pages/charts/flows.rs`
- `examples/gallery/pages/charts/mod.rs`
- `examples/gallery/pages/charts/spread.rs`
- `examples/gallery/pages/charts/wholes.rs`

### chat

공개 경계: [src/chat/mod.rs](../src/chat/mod.rs).

상태: 복합 화면·모델 API 존재, mobile 전수 증거 없음. S0/S1/S2/S3/S4/S6/S7. composer와 keyboard reveal, streaming 중 scroll 유지, long action rows, media 권한/취소 확인; AI/network/call 서비스는 host fixture.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use actions::{ BranchNavigator, MessageActions, MessageEditor, QuoteReply, continue_button, regenerate_button, stop_button, };` | `mod.rs:21` |
| `pub use attach::{Attachment, AttachmentChip, ContextChips};` | `mod.rs:25` |
| `pub use cite::{CitationBadge, Source, SourceCard, SourceList};` | `mod.rs:26` |
| `pub use code::CodeBlock;` | `mod.rs:27` |
| `pub use composer::{AttachmentButton, DragDropOverlay, InputHint, PromptInput, SendButton};` | `mod.rs:28` |
| `pub use conversations::{Conversation, ConversationItem, ConversationList, new_chat_button};` | `mod.rs:29` |
| `pub use list::{DateSeparator, MessageList, ScrollToBottomButton};` | `mod.rs:30` |
| `pub use media::{FileMessage, ImageGrid, ImageMessage, LinkPreviewCard};` | `mod.rs:31` |
| `pub use message::{ ChatContainer, MessageAvatar, MessageBubble, MessageFooter, MessageHeader, Role, };` | `mod.rs:32` |
| `pub use params::{CostEstimator, Parameter, ParameterPanel, SystemPromptEditor, TokenCounter};` | `mod.rs:35` |
| `pub use play::{AudioMessage, VideoMessage};` | `mod.rs:36` |
| `pub use projects::{ ConversationExport, Project, ProjectKnowledgePanel, ProjectList, SharedConversationView, };` | `mod.rs:37` |
| `pub use search::{DocumentChunkPreview, SearchProgress, StepState, WebResultCard};` | `mod.rs:41` |
| `pub use status::{ ErrorMessage, FeedbackForm, RateLimitNotice, ThinkingBlock, ThinkingDuration, ThinkingIndicator, };` | `mod.rs:42` |
| `pub use stream::{StreamingCursor, StreamingMarkdown, StreamingText};` | `mod.rs:45` |
| `pub use voice::{VoiceInputButton, VoiceWaveform};` | `mod.rs:46` |
| `pub use welcome::{CapabilityCards, FollowUpSuggestions, SuggestionChips, WelcomeScreen};` | `mod.rs:47` |

소스 테스트 anchor (실행 상태 아님): `src/chat/tests/welcome.rs src/chat/tests/history.rs src/chat/tests/mod.rs src/chat/tests/cite.rs src/chat/tests/compose.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/chat/blocks.rs`
- `examples/gallery/pages/chat/cite.rs`
- `examples/gallery/pages/chat/compose.rs`
- `examples/gallery/pages/chat/controls.rs`
- `examples/gallery/pages/chat/history.rs`
- `examples/gallery/pages/chat/mod.rs`
- `examples/gallery/pages/chat/talk.rs`
- `examples/gallery/pages/chat/welcome.rs`

### collab

공개 경계: [src/collab/mod.rs](../src/collab/mod.rs).

상태: 복합 화면·모델 API 존재, mobile 전수 증거 없음. S0/S1/S2/S3/S4/S6/S7. composer와 keyboard reveal, streaming 중 scroll 유지, long action rows, media 권한/취소 확인; AI/network/call 서비스는 host fixture.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use annotate::{AnnotatedText, Annotation};` | `mod.rs:12` |
| `pub use comments::{Comment, CommentMarker, CommentThread, Reply, Thread};` | `mod.rs:13` |
| `pub use peers::{FollowMode, LiveIndicator, Peer, PresenceAvatars};` | `mod.rs:14` |
| `pub use reactions::{Reaction, ReactionPicker, Reactions, toggled};` | `mod.rs:15` |
| `pub use remote::{RemoteCursor, RemoteSelection};` | `mod.rs:16` |
| `pub use share::{AccessList, InviteInput, LinkAccess, Member, PermissionSelect, Role, ShareDialog};` | `mod.rs:17` |
| `pub use sidebar::CommentSidebar;` | `mod.rs:18` |
| `pub use track::{Decision, EditMode, Suggestion, SuggestionMode, TrackChanges, accept, suggested};` | `mod.rs:19` |

소스 테스트 anchor (실행 상태 아님): `src/collab/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/collab/mod.rs`
- `examples/gallery/pages/collab/people.rs`
- `examples/gallery/pages/collab/sharing.rs`
- `examples/gallery/pages/collab/talk.rs`

### dashboard

공개 경계: [src/dashboard/mod.rs](../src/dashboard/mod.rs).

상태: 데이터 표시 UI 존재, mobile chart 탐색·dense dashboard 검증 없음. S0/S1/S2/S5/S7. hover tooltip은 tap/선택 detail 대체, 범례·축·범위 선택 접근성 확인; 실제 금융 주문/서비스 실행 없이 fixture callback 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use alerts::{ AlertList, AlertState, Incident, IncidentCard, IncidentPhase, IncidentUpdate, MonitorAlert, };` | `mod.rs:12` |
| `pub use card::DashboardCard;` | `mod.rs:15` |
| `pub use controls::{DashboardFilter, DashboardFilterBar, RefreshIntervalSelector, TimeWindow};` | `mod.rs:16` |
| `pub use events::{EventStream, StreamEvent};` | `mod.rs:17` |
| `pub use grid::DashboardGrid;` | `mod.rs:18` |
| `pub use status::{Check, Health, HealthCheck, Service, ServiceStatus, UptimeBar, overall, uptime};` | `mod.rs:19` |
| `pub use tiles::{Tile, arranged, stepped};` | `mod.rs:20` |

소스 테스트 anchor (실행 상태 아님): `src/dashboard/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/dashboard/board.rs`
- `examples/gallery/pages/dashboard/mod.rs`
- `examples/gallery/pages/dashboard/monitor.rs`

### data_display

공개 경계: [src/data_display/mod.rs](../src/data_display/mod.rs).

상태: 데이터 표시 UI 존재, mobile chart 탐색·dense dashboard 검증 없음. S0/S1/S2/S5/S7. hover tooltip은 tap/선택 detail 대체, 범례·축·범위 선택 접근성 확인; 실제 금융 주문/서비스 실행 없이 fixture callback 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use avatar::{Avatar, AvatarGroup, Presence, UserChip};` | `mod.rs:20` |
| `pub use badge::{Badge, CountBadge, DotBadge, Tag, Tone};` | `mod.rs:23` |
| `pub use before_after::BeforeAfter;` | `mod.rs:24` |
| `pub use carousel::Carousel;` | `mod.rs:25` |
| `pub use changelog::{Change, Changelog, Release};` | `mod.rs:26` |
| `pub use codes::{Barcode, BarcodeError, QrCode, QrError};` | `mod.rs:27` |
| `pub use comparison::Comparison;` | `mod.rs:28` |
| `pub use feed::{Activity, ActivityFeed};` | `mod.rs:29` |
| `pub use gallery::Gallery;` | `mod.rs:30` |
| `pub use measures::{Gauge, Meter, UsageBar};` | `mod.rs:32` |
| `pub use records::{DescriptionList, PropertyGrid, PropertyGroup};` | `mod.rs:33` |
| `pub use spark::Sparkline;` | `mod.rs:34` |
| `pub use stars::Stars;` | `mod.rs:35` |
| `pub use stat::{KpiCard, Statistic, TrendIndicator};` | `mod.rs:37` |
| `pub use timeline::{Timeline, TimelineItem};` | `mod.rs:38` |
| `pub use watermark::Watermark;` | `mod.rs:39` |

소스 테스트 anchor (실행 상태 아님): `src/data_display/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/data/measures.rs`
- `examples/gallery/pages/data/media.rs`
- `examples/gallery/pages/data/mod.rs`
- `examples/gallery/pages/data/people.rs`
- `examples/gallery/pages/data/records.rs`

### debug

공개 경계: [src/debug/mod.rs](../src/debug/mod.rs).

상태: dense desktop 작업 UI와 데이터 모델 존재; mobile 편집/hover/shortcut/PTY 계약 미검증. S0/S2/S3/S4/S6/S7. command palette/visible action과 single-pane detail 대안; terminal PTY·process·debug 실행은 OS backend 지원 없으면 명시적 미지원/원격 서비스로 분리.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use disasm::{Disassembly, Instruction};` | `mod.rs:16` |
| `pub use flame::{Flamegraph, Placed, ProfileFrame, place};` | `mod.rs:17` |
| `pub use hex::HexViewer;` | `mod.rs:18` |
| `pub use network::{NetworkInspector, Request};` | `mod.rs:19` |
| `pub use session::{Breakpoint, BreakpointList, DebugCommand, DebugState, DebugToolbar};` | `mod.rs:20` |
| `pub use stack::{CallStack, StackFrame, Thread, ThreadList, ThreadState};` | `mod.rs:21` |
| `pub use timeline::{Span, TimelineProfiler, zoomed};` | `mod.rs:23` |
| `pub use variables::{ValueKind, Variable, VariablesPanel, Watch, WatchPanel};` | `mod.rs:24` |

소스 테스트 anchor (실행 상태 아님): `src/debug/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/debug/machine.rs`
- `examples/gallery/pages/debug/mod.rs`
- `examples/gallery/pages/debug/profiling.rs`
- `examples/gallery/pages/debug/session.rs`

### devtools

공개 경계: [src/devtools/mod.rs](../src/devtools/mod.rs).

상태: dense desktop 작업 UI와 데이터 모델 존재; mobile 편집/hover/shortcut/PTY 계약 미검증. S0/S2/S3/S4/S6/S7. command palette/visible action과 single-pane detail 대안; terminal PTY·process·debug 실행은 OS backend 지원 없으면 명시적 미지원/원격 서비스로 분리.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use collection::{CollectionTree, Saved, moved};` | `mod.rs:26` |
| `pub use connection_form::{ConnectionForm, TestState};` | `mod.rs:27` |
| `pub use connections::{Connection, ConnectionManager, ConnectionState, Engine};` | `mod.rs:28` |
| `pub use contrast::{ColorContrastChecker, contrast, grades, luminance, over};` | `mod.rs:29` |
| `pub use datum::{Datum, Unread, read_json, read_yaml};` | `mod.rs:30` |
| `pub use document::{JsonViewer, YamlViewer};` | `mod.rs:31` |
| `pub use encode::{Code, Encoder, base64, hex, unbase64, unhex, unpercent};` | `mod.rs:32` |
| `pub use environment::{Environment, EnvironmentSelector};` | `mod.rs:33` |
| `pub use graphql::{GraphQLExplorer, SchemaType, TypeKind};` | `mod.rs:34` |
| `pub use json_editor::JsonEditor;` | `mod.rs:35` |
| `pub use machines::{Container, ContainerList, ContainerState, Pod, PodList, PodPhase};` | `mod.rs:36` |
| `pub use monitor::{Reading, ResourceMonitor};` | `mod.rs:37` |
| `pub use queries::{Outcome, PlanStep, QueryHistory, QueryPlanViewer, QueryRun};` | `mod.rs:38` |
| `pub use redis::{RedisKey, RedisKeyBrowser, RedisValue};` | `mod.rs:39` |
| `pub use regex::{Found, RegexFlag, RegexTester, find};` | `mod.rs:40` |
| `pub use request::{ApiRequest, ApiRequestBuilder, Auth, Method, percent, resolve};` | `mod.rs:41` |
| `pub use response::{ApiResponse, HeadersTable, ResponseViewer, reason, status_tone};` | `mod.rs:42` |
| `pub use schema::{DbTable, ERDiagram, Field, FieldKey, Schema, SchemaTree};` | `mod.rs:43` |
| `pub use socket::{Direction, SocketMessage, SocketState, WebSocketConsole};` | `mod.rs:44` |
| `pub use structure::{Index, IndexManager, TableStructureEditor};` | `mod.rs:45` |
| `pub use xml::{XmlElement, XmlNode, XmlViewer, read_xml};` | `mod.rs:46` |

소스 테스트 anchor (실행 상태 아님): `src/devtools/tests/api.rs src/devtools/tests/formats.rs src/devtools/tests/mod.rs src/devtools/tests/tools.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/devtools/apis.rs`
- `examples/gallery/pages/devtools/data.rs`
- `examples/gallery/pages/devtools/db.rs`
- `examples/gallery/pages/devtools/formats.rs`
- `examples/gallery/pages/devtools/mod.rs`
- `examples/gallery/pages/devtools/tools.rs`

### documents

공개 경계: [src/documents/mod.rs](../src/documents/mod.rs).

상태: dense desktop 작업 UI와 데이터 모델 존재; mobile 편집/hover/shortcut/PTY 계약 미검증. S0/S2/S3/S4/S6/S7. command palette/visible action과 single-pane detail 대안; terminal PTY·process·debug 실행은 OS backend 지원 없으면 명시적 미지원/원격 서비스로 분리.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use blocks::{Align, BlockData, BlockEditor, BlockEvent, BlockKind, Media};` | `mod.rs:29` |
| `pub use counts::{ReadingTime, WordCount};` | `mod.rs:30` |
| `pub use epub::{Chapter, EpubReader};` | `mod.rs:31` |
| `pub use format::{Format, format};` | `mod.rs:32` |
| `pub use history::{PageHistoryDiff, Version, VersionHistory};` | `mod.rs:34` |
| `pub use knowledge::{Backlink, Backlinks, Favorite, Favorites, TrashBin, Trashed};` | `mod.rs:35` |
| `pub use link::LinkEditor;` | `mod.rs:36` |
| `pub use markdown::markdown_highlights;` | `mod.rs:37` |
| `pub use modes::{MarkdownEditor, MarkdownMode};` | `mod.rs:38` |
| `pub use pages::{PageCover, PageIcon};` | `mod.rs:39` |
| `pub use print::{Margins, Paper, PrintPreview, PrintSettings, sheet};` | `mod.rs:40` |
| `pub use reading::{Footnote, Glossary, ReadingProgress, TooltipTerm, ZenMode};` | `mod.rs:41` |
| `pub use render::{MarkdownRenderer, outline};` | `mod.rs:42` |
| `pub use rich::RichTextEditor;` | `mod.rs:43` |
| `pub use templates::{Template, TemplatePicker};` | `mod.rs:44` |
| `pub use thumbs::PageThumbnailList;` | `mod.rs:45` |
| `pub use todo::{Checklist, TodoItem};` | `mod.rs:46` |
| `pub use toolbar::FixedFormatToolbar;` | `mod.rs:47` |
| `pub use viewer::{DocPage, DocumentViewer, PageHit, PageNote};` | `mod.rs:49` |

소스 테스트 anchor (실행 상태 아님): `src/documents/tests.rs src/documents/modes/tests.rs src/documents/blocks/tests/mod.rs src/documents/blocks/tests/stale.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/documents/editing.rs`
- `examples/gallery/pages/documents/knowledge.rs`
- `examples/gallery/pages/documents/mod.rs`
- `examples/gallery/pages/documents/reading.rs`

### editor

공개 경계: [src/editor/mod.rs](../src/editor/mod.rs).

상태: dense desktop 작업 UI와 데이터 모델 존재; mobile 편집/hover/shortcut/PTY 계약 미검증. S0/S2/S3/S4/S6/S7. command palette/visible action과 single-pane detail 대안; terminal PTY·process·debug 실행은 OS backend 지원 없으면 명시적 미지원/원격 서비스로 분리.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use buffer::Buffer;` | `mod.rs:36` |
| `pub use chat::InlineChat;` | `mod.rs:37` |
| `pub use console::{ConsoleEntry, DebugConsole, OutputPanel};` | `mod.rs:38` |
| `pub use cursor::Selection;` | `mod.rs:39` |
| `pub use decor::{ CodeLens, Diagnostic, DiffHunk, GhostText, GitMark, InlayHint, ReadOnlyBanner, VimMode, VimModeIndicator, };` | `mod.rs:40` |
| `pub use extensions::{Extension, ExtensionAction, ExtensionState, ExtensionsPanel};` | `mod.rs:44` |
| `pub use filters::{SearchFilters, passes};` | `mod.rs:45` |
| `pub use find::{FindOptions, FindWidget, find_all};` | `mod.rs:46` |
| `pub use hints::{CompletionItem, CompletionMenu, Signature, SignatureHelp, completions};` | `mod.rs:47` |
| `pub use hover::HoverInfo;` | `mod.rs:48` |
| `pub use keybindings::{KeySource, Keybinding, KeybindingsEditor, conflicts};` | `mod.rs:49` |
| `pub use preview::ThemePreview;` | `mod.rs:52` |
| `pub use problems::{Problem, ProblemsPanel};` | `mod.rs:53` |
| `pub use refactor::{ActionKind, CodeAction, CodeActionMenu, RenameInput};` | `mod.rs:54` |
| `pub use search::{FileHits, Hit, ReferencesPanel, SearchPanel, SearchResultItem};` | `mod.rs:55` |
| `pub use settings::{Setting, SettingValue, SettingsEditor, settings_json};` | `mod.rs:56` |
| `pub use state::{CodeEditor, CursorShape, EditorEvent, LineNumbers};` | `mod.rs:57` |
| `pub use status::{ BranchIndicator, CursorPosition, Encoding, IndentSettings, LanguageMode, LineEnding, LspState, LspStatus, NotificationBell, };` | `mod.rs:58` |
| `pub use symbols::{Call, CallHierarchy, GoToSymbol, Symbol, SymbolKind, SymbolOutline};` | `mod.rs:62` |
| `pub use tasks::{Task, TaskRunner, TaskState};` | `mod.rs:64` |
| `pub use welcome::{Project, ProjectSwitcher, RecentProjects, WelcomePage};` | `mod.rs:65` |

소스 테스트 anchor (실행 상태 아님): `src/editor/tests/mod.rs src/editor/tests/stale.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/editor/code.rs`
- `examples/gallery/pages/editor/intel.rs`
- `examples/gallery/pages/editor/mod.rs`
- `examples/gallery/pages/editor/panels.rs`
- `examples/gallery/pages/editor/search.rs`
- `examples/gallery/pages/editor/settings.rs`
- `examples/gallery/pages/editor/start.rs`
- `examples/gallery/pages/editor/status.rs`

### feedback

공개 경계: [src/feedback/mod.rs](../src/feedback/mod.rs).

상태: 표시·상태 전환 UI 존재, touch dismissal/lifecycle 검증 없음. S0/S1/S4/S7. transient 메시지는 읽기 시간·screen reader 확인; hover 도움은 tap과 영구 도움 대체, reduced-motion 및 background timer 확인.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use clock::{Countdown, Timer};` | `mod.rs:14` |
| `pub use confirm::ConfirmationCard;` | `mod.rs:15` |
| `pub use messages::{Alert, Banner, Callout, InlineMessage, StatusMessage};` | `mod.rs:16` |
| `pub use notification::{Notification, NotificationCenter};` | `mod.rs:17` |
| `pub use result::ResultView;` | `mod.rs:18` |
| `pub use states::{EmptyState, ErrorBoundary, ErrorView};` | `mod.rs:19` |
| `pub use status::{ConnectionStatus, SaveState, SavingIndicator, SyncState, SyncStatus};` | `mod.rs:20` |
| `pub use suspense::{AsyncView, Suspense};` | `mod.rs:21` |
| `pub use toast::{Toast, ToastViewport, Toaster};` | `mod.rs:22` |

소스 테스트 anchor (실행 상태 아님): `src/feedback/tests/states.rs src/feedback/tests/mod.rs src/feedback/tests/loading.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/feedback/messages.rs`
- `examples/gallery/pages/feedback/mod.rs`
- `examples/gallery/pages/feedback/states.rs`

### files

공개 경계: [src/files/mod.rs](../src/files/mod.rs).

상태: 표시/조작 API 존재; camera/audio/video/native view/file 서비스 native 연결 보장 없음. S0/S1/S2/S5/S6/S7. 파일 drop은 picker, hover 툴은 visible controls; permission 거부와 unavailable 상태까지 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use archive::{ArchiveEntry, ArchiveViewer};` | `mod.rs:26` |
| `pub use duplicates::{DuplicateFinder, Duplicates};` | `mod.rs:27` |
| `pub use explorer::{FileExplorer, FileView};` | `mod.rs:28` |
| `pub use grid::FileGrid;` | `mod.rs:29` |
| `pub use icon::FileIcon;` | `mod.rs:30` |
| `pub use item::FileItem;` | `mod.rs:31` |
| `pub use operation::{FileOperation, FileOperationProgress};` | `mod.rs:32` |
| `pub use preview::FilePreview;` | `mod.rs:33` |
| `pub use recent::{RecentFile, RecentFiles};` | `mod.rs:34` |
| `pub use search::{FileSearch, FoundFile};` | `mod.rs:35` |
| `pub use transfer::{Transfer, TransferState};` | `mod.rs:36` |
| `pub use transfers::{DownloadManager, TransferQueue};` | `mod.rs:37` |

소스 테스트 anchor (실행 상태 아님): `src/files/tests/explorer.rs src/files/tests/find.rs src/files/tests/mod.rs src/files/tests/stale.rs src/files/tests/transfers.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/files/actions.rs`
- `examples/gallery/pages/files/explorer.rs`
- `examples/gallery/pages/files/find.rs`
- `examples/gallery/pages/files/mod.rs`
- `examples/gallery/pages/files/transfers.rs`

### finance

공개 경계: [src/finance/mod.rs](../src/finance/mod.rs).

상태: 데이터 표시 UI 존재, mobile chart 탐색·dense dashboard 검증 없음. S0/S1/S2/S5/S7. hover tooltip은 tap/선택 detail 대체, 범례·축·범위 선택 접근성 확인; 실제 금융 주문/서비스 실행 없이 fixture callback 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use blotter::{ Fill, OrderTable, PnLDisplay, Position, PositionTable, TradeHistoryTable, Working, };` | `mod.rs:31` |
| `pub use book::{Level2Quotes, OrderBook, Side, Trade};` | `mod.rs:34` |
| `pub use calendars::{Earnings, EarningsCalendar, EconomicCalendar, NewsFeed, Release, Story};` | `mod.rs:35` |
| `pub use candles::Candle;` | `mod.rs:36` |
| `pub use controls::{ Arrangement, ChartTypeSwitcher, INTERVALS, IntervalSelector, MultiChartLayout, RANGES, TimeRangeSelector, };` | `mod.rs:37` |
| `pub use depth::DepthChart;` | `mod.rs:41` |
| `pub use heatmap::MarketHeatmap;` | `mod.rs:42` |
| `pub use market::{CandlestickChart, ChartKind, ChartSync};` | `mod.rs:43` |
| `pub use options::{GreeksTable, OptionChain, OptionQuote, Strike};` | `mod.rs:44` |
| `pub use overview::{MarketOverview, MarketStatus, Session, TradingSessionClock};` | `mod.rs:45` |
| `pub use payoff::{Leg, PayoffDiagram};` | `mod.rs:46` |
| `pub use portfolio::{ Dividend, DividendTable, FinancialStatementTable, PerformanceChart, PortfolioSummary, Statement, };` | `mod.rs:47` |
| `pub use quotes::{PriceChangeBadge, PriceText, QuoteCard, TickerTape};` | `mod.rs:50` |
| `pub use renko::{PointFigureChart, RenkoChart};` | `mod.rs:51` |
| `pub use risk::{LeverageSlider, MarginIndicator, RiskMeter};` | `mod.rs:52` |
| `pub use series::{Overlay, Study};` | `mod.rs:53` |
| `pub use spread::{BidAskBar, SpreadIndicator};` | `mod.rs:54` |
| `pub use symbols::{Screener, SymbolBadge, SymbolSearch, Watch, Watchlist};` | `mod.rs:55` |
| `pub use tape::{DomLadder, TimeAndSales};` | `mod.rs:56` |
| `pub use tools::{Drawing, DrawingToolbar, FIB, IndicatorSelector, Tool};` | `mod.rs:57` |
| `pub use trade::{Order, OrderConfirmDialog, OrderEntry, OrderKind, QuickTradeButtons};` | `mod.rs:58` |
| `pub use wallet::{CryptoWalletCard, CurrencyConverter, Settled, TransactionList, Transfer};` | `mod.rs:59` |

소스 테스트 anchor (실행 상태 아님): `src/finance/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/finance/assets.rs`
- `examples/gallery/pages/finance/book.rs`
- `examples/gallery/pages/finance/markets.rs`
- `examples/gallery/pages/finance/mod.rs`
- `examples/gallery/pages/finance/overview.rs`
- `examples/gallery/pages/finance/trading.rs`

### forms

공개 경계: [src/forms/mod.rs](../src/forms/mod.rs).

E-CORE-HOST에서 이 라인의 forms 필터가 통과했다. 선택한 Checkbox/Radio/Switch/Select 및 Slider/RangeSlider의 touch 회귀를 포함하지만 모든 compound form의 native 동작을 인증하지 않는다.

상태: 입력/선택 UI 존재, native IME 및 각 compound form 검증 없음. S0/S1/S2/S3/S4/S7. keyboard-only shortcut/scrub에 버튼 또는 직접 입력 대안; 업로드·인증·전송은 host 서비스이며 fixture action만 사용.

FormField→FormLabel의 창별 bounds 전달과 이름 있는 touch help는 소스 동결 상태이며 새 테스트 실행 결과는 아직 기록하지 않는다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use avatar::AvatarUpload;` | `mod.rs:48` |
| `pub use card::{CheckboxCard, RadioCard};` | `mod.rs:49` |
| `pub use cascade::{Cascade, Cascader};` | `mod.rs:50` |
| `pub use catalogs::FontPicker;` | `mod.rs:51` |
| `pub use check::{CheckState, Checkbox, CheckboxGroup};` | `mod.rs:54` |
| `pub use checked::{EmailInput, UrlInput, is_email, is_url};` | `mod.rs:55` |
| `pub use chips::ChoiceChips;` | `mod.rs:56` |
| `pub use code::{CodeInput, JsonInput, code_highlights, json_highlights};` | `mod.rs:57` |
| `pub use color::{ColorPalette, ColorPicker, ColorSwatch, EyeDropper, GradientEditor, GradientStop};` | `mod.rs:59` |
| `pub use combobox::Combobox;` | `mod.rs:61` |
| `pub use date::{ Calendar, CronEditor, CronRule, DatePicker, DateRangePicker, DateTimePicker, DurationPicker, MonthPicker, QuarterPicker, RelativeDatePicker, RelativeRange, TimePicker, TimezoneSelect, WeekPicker, YearPicker, };` | `mod.rs:62` |
| `pub use expression::{ExpressionInput, evaluate, expression_highlights};` | `mod.rs:70` |
| `pub use files::{DropZone, FileInput, PICTURES};` | `mod.rs:71` |
| `pub use glyphs::{EmojiPicker, IconPicker};` | `mod.rs:74` |
| `pub use group::{InputAddon, InputGroup};` | `mod.rs:75` |
| `pub use hotkey::HotkeyInput;` | `mod.rs:76` |
| `pub use inline::InlineEdit;` | `mod.rs:78` |
| `pub use input::{Input, PasswordInput};` | `mod.rs:79` |
| `pub use knob::Knob;` | `mod.rs:80` |
| `pub use listbox::ListBox;` | `mod.rs:81` |
| `pub use masked::{COUNTRIES, MaskedInput, PhoneInput};` | `mod.rs:82` |
| `pub use mention::{MentionInput, mention_highlights};` | `mod.rs:83` |
| `pub use multi::MultiSelect;` | `mod.rs:87` |
| `pub use number::{NumberInput, ScrubInput};` | `mod.rs:89` |
| `pub use options::Choice;` | `mod.rs:90` |
| `pub use pairs::{FieldArray, KeyValueInput, ListInput};` | `mod.rs:95` |
| `pub use path::PathInput;` | `mod.rs:96` |
| `pub use pattern::{RegexInput, regex_highlights};` | `mod.rs:98` |
| `pub use pin::PinInput;` | `mod.rs:100` |
| `pub use radio::{Radio, RadioGroup};` | `mod.rs:101` |
| `pub use rating::Rating;` | `mod.rs:102` |
| `pub use search::SearchInput;` | `mod.rs:103` |
| `pub use select::Select;` | `mod.rs:104` |
| `pub use signature::{SignaturePad, Stroke};` | `mod.rs:107` |
| `pub use slider::{RangeSlider, Slider};` | `mod.rs:109` |
| `pub use stepper::Stepper;` | `mod.rs:110` |
| `pub use structure::{ DirtyIndicator, Form, FormError, FormField, FormLabel, FormSection, InlineForm, };` | `mod.rs:111` |
| `pub use switch::Switch;` | `mod.rs:114` |
| `pub use tags::TagInput;` | `mod.rs:115` |
| `pub use text::{Highlight, History, InputEvent, TextInput};` | 공개 후보 기준. 로컬 `mod.rs:119`의 추가 `TextInputOwner` export는 미공개 IME 통합 변경으로 제외 |
| `pub use transfer::TransferList;` | `mod.rs:120` |
| `pub use unit::UnitInput;` | `mod.rs:121` |
| `pub use upload::{Upload, UploadList, UploadState};` | `mod.rs:122` |

소스 테스트 anchor (실행 상태 아님): `src/forms/text/owner/tests.rs src/forms/number/tests.rs src/forms/tests/structure.rs src/forms/tests/dates.rs src/forms/tests/choices.rs src/forms/tests/colors.rs src/forms/tests/mod.rs src/forms/tests/stale.rs src/forms/tests/tags.rs src/forms/tests/upload.rs src/forms/tests/other.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/forms/choices.rs`
- `examples/gallery/pages/forms/colors.rs`
- `examples/gallery/pages/forms/dates.rs`
- `examples/gallery/pages/forms/files.rs`
- `examples/gallery/pages/forms/formats.rs`
- `examples/gallery/pages/forms/mod.rs`
- `examples/gallery/pages/forms/numbers.rs`
- `examples/gallery/pages/forms/other.rs`
- `examples/gallery/pages/forms/pickers.rs`
- `examples/gallery/pages/forms/rich.rs`
- `examples/gallery/pages/forms/structure.rs`
- `examples/gallery/pages/forms/text.rs`
- `examples/gallery/pages/forms/values.rs`

### generative

공개 경계: [src/generative/mod.rs](../src/generative/mod.rs).

상태: 복합 화면·모델 API 존재, mobile 전수 증거 없음. S0/S1/S2/S3/S4/S6/S7. composer와 keyboard reveal, streaming 중 scroll 유지, long action rows, media 권한/취소 확인; AI/network/call 서비스는 host fixture.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use audio::AudioGenerationPlayer;` | `mod.rs:25` |
| `pub use brush::MaskBrush;` | `mod.rs:26` |
| `pub use compare::{ABCompareView, Verdict};` | `mod.rs:27` |
| `pub use downloads::{Download, DownloadState, ModelDownloadManager};` | `mod.rs:28` |
| `pub use embed::{Embedded, EmbeddingVisualizer};` | `mod.rs:29` |
| `pub use grid::{GenerationGrid, Outcome};` | `mod.rs:30` |
| `pub use hardware::{HardwareMonitor, Reading};` | `mod.rs:31` |
| `pub use mask::{InpaintCanvas, MaskStroke};` | `mod.rs:32` |
| `pub use models::{ModelCard, ModelState, ModelStatus};` | `mod.rs:33` |
| `pub use playground::PromptPlayground;` | `mod.rs:34` |
| `pub use prompt::PromptEnhancer;` | `mod.rs:35` |
| `pub use queue::{GenerationQueue, Job, JobState};` | `mod.rs:36` |
| `pub use ratio::AspectRatioPicker;` | `mod.rs:37` |
| `pub use seed::SeedInput;` | `mod.rs:38` |
| `pub use style::{StylePreset, StylePresetPicker};` | `mod.rs:39` |
| `pub use tuning::{FineTuneJobCard, TunePhase};` | `mod.rs:40` |
| `pub use variation::VariationPicker;` | `mod.rs:41` |
| `pub use versions::{PromptDiff, PromptVersion, PromptVersionHistory};` | `mod.rs:42` |
| `pub use video::{Shot, VideoGenerationTimeline};` | `mod.rs:43` |
| `pub use voice::{TTSVoicePicker, Voice};` | `mod.rs:44` |

소스 테스트 anchor (실행 상태 아님): `src/generative/tests/settings.rs src/generative/tests/canvas.rs src/generative/tests/data.rs src/generative/tests/mod.rs src/generative/tests/stale.rs src/generative/tests/sound.rs src/generative/tests/models.rs src/generative/tests/results.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/generative/canvas.rs`
- `examples/gallery/pages/generative/data.rs`
- `examples/gallery/pages/generative/mod.rs`
- `examples/gallery/pages/generative/models.rs`
- `examples/gallery/pages/generative/panel.rs`
- `examples/gallery/pages/generative/results.rs`
- `examples/gallery/pages/generative/sound.rs`

### git

공개 경계: [src/git/mod.rs](../src/git/mod.rs).

상태: dense desktop 작업 UI와 데이터 모델 존재; mobile 편집/hover/shortcut/PTY 계약 미검증. S0/S2/S3/S4/S6/S7. command palette/visible action과 single-pane detail 대안; terminal PTY·process·debug 실행은 OS backend 지원 없으면 명시적 미지원/원격 서비스로 분리.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use badges::{DiffStat, GitStatusBadge};` | `mod.rs:18` |
| `pub use blame::{Blame, BlameView, GitBlameAnnotation};` | `mod.rs:19` |
| `pub use changes::{ChangeAction, ChangeSection, Changed, ChangesList, CommitInput};` | `mod.rs:20` |
| `pub use commits::{Commit, CommitItem, CommitList};` | `mod.rs:21` |
| `pub use diff::{DiffLine, LineKind, Stretch, diff, mark_words, pairs, stat};` | `mod.rs:22` |
| `pub use graph::{GraphRow, Half, Stroke, lanes};` | `mod.rs:23` |
| `pub use history::FileHistory;` | `mod.rs:24` |
| `pub use merge::{ConflictResolver, ThreeWayMerge};` | `mod.rs:25` |
| `pub use merging::{ Conflict, MARKER, Region, RegionKind, Take, conflicts, marked, regions, resolve, result, };` | `mod.rs:26` |
| `pub use refs::{Branch, BranchList, BranchSelector};` | `mod.rs:29` |
| `pub use review::{PullRequest, PullRequestCard, PullState, ReviewComment, ReviewNote};` | `mod.rs:30` |
| `pub use rows::Place;` | `mod.rs:31` |
| `pub use tags::{GitTag, Stash, StashAction, StashList, TagList};` | `mod.rs:32` |
| `pub use viewer::{DiffLayout, DiffViewer};` | `mod.rs:33` |

소스 테스트 anchor (실행 상태 아님): `src/git/tests/picking.rs src/git/tests/mod.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/git/diffs.rs`
- `examples/gallery/pages/git/mod.rs`
- `examples/gallery/pages/git/people.rs`
- `examples/gallery/pages/git/work.rs`

### interaction

공개 경계: [src/interaction/mod.rs](../src/interaction/mod.rs).

상태: gesture/geometry API 존재; E-CANVAS의 좁은 drag 외 도구별 native 증거 없음. S0/S1/S2/S5/S7. resize/rotate/selection handles는 touch hit 영역과 cancel 복구 확인; mouse-wheel-only 조작은 +/- 및 reset 대안. 지도 GPS는 별도 서비스.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use resizable::Resizable;` | `mod.rs:10` |
| `pub use rotatable::Rotatable;` | `mod.rs:11` |
| `pub use roving::RovingFocus;` | `mod.rs:12` |
| `pub use selection::SelectionArea;` | `mod.rs:13` |
| `pub use sync::ScrollSync;` | `mod.rs:14` |

소스 테스트 anchor (실행 상태 아님): `src/interaction/tests/mod.rs src/interaction/tests/limits.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/interaction/keys.rs`
- `examples/gallery/pages/interaction/mod.rs`
- `examples/gallery/pages/interaction/pointer.rs`

### layout

공개 경계: [src/layout/mod.rs](../src/layout/mod.rs).

상태: geometry helper·Sheet/Dialog bounds와 Sheet focus/scroll 구현; E-LAYOUT 외 컴포넌트별 native 증거 없음. S0/S2/S3/S4/S7. Dock/resize/floating desktop 배치는 compact single-pane + navigation/Sheet로 대체; HoverCard/Peek는 tap 접근 제공 여부 확인.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use collapse::{Accordion, AccordionItem, Collapsible};` | `mod.rs:23` |
| `pub use dock::{Dock, DockLayout, DockPanel, DockSide, Spot};` | `mod.rs:24` |
| `pub use floating::FloatingPanel;` | `mod.rs:25` |
| `pub use grid::{Masonry, SimpleGrid};` | `mod.rs:26` |
| `pub use page::{AppShell, MasterDetail, Page};` | `mod.rs:28` |
| `pub use panes::{PaneGroup, PaneId, PaneLayout};` | `mod.rs:29` |
| `pub use responsive::{ViewportInsets, is_compact, reveal_in_bounds, reveal_offset, usable_bounds};` | `mod.rs:30` |
| `pub use scroll::{ScrollArea, Scrollbar, on_axis, reveal_when_focused_in_bounds};` | `mod.rs:31` |
| `pub use scroll_aids::{ScrollShadow, ScrollToTop, StickyHeader};` | `mod.rs:33` |
| `pub use sheet::{Drawer, Sheet};` | `mod.rs:34` |
| `pub use sidebar::Sidebar;` | `mod.rs:35` |
| `pub use split::{SplitPane, resize_edge, resize_handle};` | `mod.rs:36` |
| `pub use stack::{AspectRatio, Container, ZStack, h_stack, v_stack};` | `mod.rs:37` |
| `pub use surface::{Card, CardHeader, Fieldset, Frame, Panel, Section, Well};` | `mod.rs:38` |
| `pub use viewport::{Transform, Viewport};` | `mod.rs:39` |
| `pub use workspace::{WORKSPACE_VERSION, Workspace};` | `mod.rs:40` |

소스 테스트 anchor (실행 상태 아님): `src/layout/tests.rs src/layout/responsive_tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/layout/basics.rs`
- `examples/gallery/pages/layout/mod.rs`
- `examples/gallery/pages/layout/regions.rs`
- `examples/gallery/pages/layout/scrolling.rs`
- `examples/gallery/pages/layout/shells.rs`
- `examples/gallery/pages/layout/systems.rs`

### lists

공개 경계: [src/lists/mod.rs](../src/lists/mod.rs).

상태: 목록/virtualization/selection UI가 존재한다. Sortable의 명시적 up/down과 SwipeableListItem의 containment/cancel은 E-CORE-HOST로 검증했다. Native scroll·drag·cell edit 전수 증거는 없다. S0/S1/S2/S3/S5/S7을 유지하며 넓은 표에는 column 선택/세부 화면을 제공하고, 읽기전용 표시와 편집 가능 상태를 별도로 검증한다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use directory::{DirEntry, DirectoryListing};` | `mod.rs:15` |
| `pub use files::{FileTree, GitStatus};` | `mod.rs:18` |
| `pub use grouped::GroupedList;` | `mod.rs:19` |
| `pub use item::{List, ListItem};` | `mod.rs:20` |
| `pub use long::{InfiniteList, VirtualList};` | `mod.rs:21` |
| `pub use outline::Outline;` | `mod.rs:22` |
| `pub use select::SelectableList;` | `mod.rs:24` |
| `pub use sortable::SortableList;` | `mod.rs:27` |
| `pub use swipe::{SwipeAction, SwipeableListItem};` | `mod.rs:28` |
| `pub use tree::{DropAt, Tree, TreeNode, TreeSelect};` | `mod.rs:29` |

소스 테스트 anchor (실행 상태 아님): `src/lists/tests/mod.rs src/lists/tests/keys.rs src/lists/tree/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/lists/files.rs`
- `examples/gallery/pages/lists/long.rs`
- `examples/gallery/pages/lists/mod.rs`
- `examples/gallery/pages/lists/rows.rs`
- `examples/gallery/pages/lists/trees.rs`

### mail

공개 경계: [src/mail/mod.rs](../src/mail/mod.rs).

상태: 입력/선택 UI 존재, native IME 및 각 compound form 검증 없음. S0/S1/S2/S3/S4/S7. keyboard-only shortcut/scrub에 버튼 또는 직접 입력 대안; 업로드·인증·전송은 host 서비스이며 fixture action만 사용.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use boxes::{Mailbox, MailboxList};` | `mod.rs:16` |
| `pub use compose::{Draft, MailComposer};` | `mod.rs:17` |
| `pub use labels::{Label, LabelPicker};` | `mod.rs:18` |
| `pub use list::{Mail, MailItem, MailList};` | `mod.rs:19` |
| `pub use quoted::QuotedText;` | `mod.rs:20` |
| `pub use reader::{Contact, MailReader, Message};` | `mod.rs:21` |
| `pub use send::ScheduleSend;` | `mod.rs:22` |
| `pub use signature::SignatureEditor;` | `mod.rs:23` |
| `pub use snooze::SnoozePicker;` | `mod.rs:24` |
| `pub use thread::MailThreadView;` | `mod.rs:25` |

소스 테스트 anchor (실행 상태 아님): `src/mail/tests/read.rs src/mail/tests/mod.rs src/mail/tests/write.rs src/mail/tests/sort.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/mail/mod.rs`
- `examples/gallery/pages/mail/read.rs`
- `examples/gallery/pages/mail/sort.rs`
- `examples/gallery/pages/mail/write.rs`

### maps

공개 경계: [src/maps/mod.rs](../src/maps/mod.rs).

상태: gesture/geometry API 존재; E-CANVAS의 좁은 drag 외 도구별 native 증거 없음. S0/S1/S2/S5/S7. resize/rotate/selection handles는 touch hit 영역과 cancel 복구 확인; mouse-wheel-only 조작은 +/- 및 reset 대안. 지도 GPS는 별도 서비스.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use choropleth::{ChoroplethKey, ChoroplethLayer, ChoroplethMap};` | `mod.rs:16` |
| `pub use coords::{CoordFormat, CoordinateDisplay};` | `mod.rs:17` |
| `pub use geo::{LatLon, MAX_LAT, MapViewport, Tile};` | `mod.rs:18` |
| `pub use geojson::{Feature, Geometry, read_geojson};` | `mod.rs:19` |
| `pub use layer::{GeoHeatmap, GeoJsonLayer, MapLayer, RouteLine};` | `mod.rs:20` |
| `pub use marker::{MapMarker, MapPopup};` | `mod.rs:21` |
| `pub use picker::LocationPicker;` | `mod.rs:22` |
| `pub use view::MapView;` | `mod.rs:23` |
| `pub use world::WorldMap;` | `mod.rs:24` |

소스 테스트 anchor (실행 상태 아님): `src/maps/tests/layers.rs src/maps/tests/mod.rs src/maps/tests/popup.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/maps/layers.rs`
- `examples/gallery/pages/maps/mod.rs`
- `examples/gallery/pages/maps/world.rs`

### media

공개 경계: [src/media/mod.rs](../src/media/mod.rs).

상태: 표시/조작 API 존재; camera/audio/video/native view/file 서비스 native 연결 보장 없음. S0/S1/S2/S5/S6/S7. 파일 drop은 picker, hover 툴은 visible controls; permission 거부와 unavailable 상태까지 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use annotate::ImageAnnotator;` | `mod.rs:25` |
| `pub use audio::{AudioSpectrum, AudioWaveform};` | `mod.rs:26` |
| `pub use camera::{CameraPreview, Device, DeviceKind, DeviceSelector};` | `mod.rs:28` |
| `pub use controls::{MediaControls, PlaybackSpeedControl, Repeat, VolumeControl};` | `mod.rs:29` |
| `pub use crop::Crop;` | `mod.rs:30` |
| `pub use cropper::ImageCropper;` | `mod.rs:31` |
| `pub use listen::{AudioPlayer, Playlist, Track};` | `mod.rs:32` |
| `pub use marks::{Mark, Tool};` | `mod.rs:33` |
| `pub use model::{ModelViewer, Orbit};` | `mod.rs:34` |
| `pub use player::{SPEEDS, VideoPlayer};` | `mod.rs:35` |
| `pub use record::{MicLevelMeter, Recording, ScreenRecorderControls};` | `mod.rs:37` |
| `pub use scrubber::Scrubber;` | `mod.rs:38` |
| `pub use strip::VideoThumbnailStrip;` | `mod.rs:39` |
| `pub use subtitles::{Cue, SubtitleEditor};` | `mod.rs:40` |
| `pub use thumbnail::ImageThumbnail;` | `mod.rs:41` |
| `pub use upload::ImageUpload;` | `mod.rs:42` |
| `pub use viewer::ImageViewer;` | `mod.rs:43` |

소스 테스트 anchor (실행 상태 아님): `src/media/tests/player.rs src/media/tests/images.rs src/media/tests/audio.rs src/media/tests/mod.rs src/media/tests/devices.rs src/media/tests/video.rs src/media/tests/stale.rs src/media/tests/edit.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/media/audio.rs`
- `examples/gallery/pages/media/devices.rs`
- `examples/gallery/pages/media/edit.rs`
- `examples/gallery/pages/media/images.rs`
- `examples/gallery/pages/media/mod.rs`
- `examples/gallery/pages/media/sculpt.rs`
- `examples/gallery/pages/media/video.rs`

### menus

공개 경계: [src/menus/mod.rs](../src/menus/mod.rs).

상태: bounds/scroll/wheel containment 구현, 해당 과거 snapshot의 실행 증거. MenuBar/PieMenu/context/searchable 전체 native 검증은 남음. S0/S1/S2/S4/S7; context/right-click/hover 의존 기능에는 visible trigger 또는 기존 ActionSheet 대체가 필요.

MenuBar의 매 render bounds 전달, root/submenu 제약과 no-room guard 및 gallery 연결은 소스 동결 상태다. 새 테스트 실행은 별도이며 기존 실행 증거는 E-MENU/E-OVERLAY-HOST의 과거 snapshot에 한정된다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use bar::MenuBar;` | `mod.rs:11` |
| `pub use hosts::{ContextMenu, DropdownMenu, OverflowMenu, SearchableMenu, SplitButton};` | `mod.rs:13` |
| `pub use model::{Menu, MenuItem};` | `mod.rs:14` |
| `pub use pie::{PieItem, PieMenu};` | `mod.rs:15` |

소스 테스트 anchor (실행 상태 아님): `src/menus/tests/more.rs src/menus/tests/mod.rs src/menus/tests/scroll.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/menus/mod.rs`
- `examples/gallery/pages/menus/more.rs`
- `examples/gallery/pages/menus/rows.rs`

### messaging

공개 경계: [src/messaging/mod.rs](../src/messaging/mod.rs).

상태: 복합 화면·모델 API 존재, mobile 전수 증거 없음. S0/S1/S2/S3/S4/S6/S7. composer와 keyboard reveal, streaming 중 scroll 유지, long action rows, media 권한/취소 확인; AI/network/call 서비스는 host fixture.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use call::{CallControls, HuddleIndicator, VoiceCallBar};` | `mod.rs:19` |
| `pub use channels::{ChannelItem, ChannelList, ChatRow, DirectMessageItem};` | `mod.rs:20` |
| `pub use gifs::{Gif, GifPicker, Gifs};` | `mod.rs:21` |
| `pub use header::ChannelHeader;` | `mod.rs:22` |
| `pub use incoming::IncomingCallDialog;` | `mod.rs:23` |
| `pub use members::{Member, MemberList};` | `mod.rs:24` |
| `pub use messages::{ChatMessage, UnreadDivider};` | `mod.rs:25` |
| `pub use people::{OnlineStatus, UserProfileCard};` | `mod.rs:26` |
| `pub use pins::{PinnedMessage, PinnedMessages};` | `mod.rs:27` |
| `pub use receipt::{Delivery, ReadReceipt};` | `mod.rs:28` |
| `pub use status::{ClearAfter, Status, StatusSetter};` | `mod.rs:29` |
| `pub use stickers::{Sticker, StickerPack, StickerPicker};` | `mod.rs:30` |
| `pub use thread::{MessageThread, ThreadPanel};` | `mod.rs:31` |
| `pub use tiles::{ParticipantTile, ScreenShareView, VideoCallGrid};` | `mod.rs:32` |

소스 테스트 anchor (실행 상태 아님): `src/messaging/tests/people.rs src/messaging/tests/messages.rs src/messaging/tests/pickers.rs src/messaging/tests/mod.rs src/messaging/tests/calls.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/messaging/calls.rs`
- `examples/gallery/pages/messaging/messages.rs`
- `examples/gallery/pages/messaging/mod.rs`
- `examples/gallery/pages/messaging/people.rs`
- `examples/gallery/pages/messaging/pickers.rs`

### misc

공개 경계: [src/misc/mod.rs](../src/misc/mod.rs).

상태: 표시/조작 API 존재; camera/audio/video/native view/file 서비스 native 연결 보장 없음. S0/S1/S2/S5/S6/S7. 파일 drop은 picker, hover 툴은 visible controls; permission 거부와 unavailable 상태까지 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use calculator::Calculator;` | `mod.rs:21` |
| `pub use captcha::{Captcha, CaptchaState};` | `mod.rs:22` |
| `pub use clock::{Clock, WorldClock};` | `mod.rs:23` |
| `pub use consent::ConsentDialog;` | `mod.rs:24` |
| `pub use cookies::CookieBanner;` | `mod.rs:25` |
| `pub use csv::{CsvImporter, read_csv};` | `mod.rs:26` |
| `pub use export::{ExportDialog, ExportFormat};` | `mod.rs:27` |
| `pub use flashcards::Flashcards;` | `mod.rs:28` |
| `pub use import::{ImportDialog, ImportField};` | `mod.rs:29` |
| `pub use licenses::{LicenseViewer, Package};` | `mod.rs:30` |
| `pub use poll::Poll;` | `mod.rs:31` |
| `pub use quiz::{Quiz, QuizQuestion};` | `mod.rs:32` |
| `pub use scanner::QrCodeScanner;` | `mod.rs:33` |
| `pub use stopwatch::Stopwatch;` | `mod.rs:34` |
| `pub use survey::{Answer, Question, Survey};` | `mod.rs:35` |
| `pub use units::UnitConverter;` | `mod.rs:36` |
| `pub use web::{IframeEmbed, WebSource, WebView};` | `mod.rs:37` |

소스 테스트 anchor (실행 상태 아님): `src/misc/tests/notices.rs src/misc/tests/transfer.rs src/misc/tests/asking.rs src/misc/tests/mod.rs src/misc/tests/codes.rs src/misc/tests/numbers.rs src/misc/tests/drift.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/misc/asking.rs`
- `examples/gallery/pages/misc/codes.rs`
- `examples/gallery/pages/misc/mod.rs`
- `examples/gallery/pages/misc/notices.rs`
- `examples/gallery/pages/misc/transfer.rs`
- `examples/gallery/pages/misc/web.rs`

### motion

공개 경계: [src/motion/mod.rs](../src/motion/mod.rs).

상태: 표시·상태 전환 UI 존재, touch dismissal/lifecycle 검증 없음. S0/S1/S4/S7. transient 메시지는 읽기 시간·screen reader 확인; hover 도움은 tap과 영구 도움 대체, reduced-motion 및 background timer 확인.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use curve::*;` | `mod.rs:18` |
| `pub use effects::{Blink, Flash, Glow, Marquee, Pulse, Shake};` | `mod.rs:20` |
| `pub use list::{AnimatePresence, Flip};` | `mod.rs:21` |
| `pub use overlay::{LazyLoad, LoadingOverlay, Refresh, RefreshIndicator, TypingIndicator};` | `mod.rs:22` |
| `pub use particles::{AnimatedGradient, Confetti, ParticleBackground, Ripple};` | `mod.rs:23` |
| `pub use progress::{ProgressBar, ProgressRing};` | `mod.rs:25` |
| `pub use reorder::Reorder;` | `mod.rs:26` |
| `pub use skeleton::{Shimmer, Skeleton, SkeletonAvatar, SkeletonCard, SkeletonTable, SkeletonText};` | `mod.rs:27` |
| `pub use spinner::{Spinner, SpinnerStyle};` | `mod.rs:30` |
| `pub use transition::{Entrance, Stagger, Transition};` | `mod.rs:31` |

소스 테스트 anchor (실행 상태 아님): `src/motion/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/motion/effects.rs`
- `examples/gallery/pages/motion/loading.rs`
- `examples/gallery/pages/motion/mod.rs`
- `examples/gallery/pages/motion/moves.rs`
- `examples/gallery/pages/motion/progress.rs`

### navigation

공개 경계: [src/navigation/mod.rs](../src/navigation/mod.rs).

Tabs의 visible context와 EditorTabs의 close가 selection으로 전파되지 않는지는 E-CORE-HOST에서 검증했다. 이 결과를 다른 navigation 컴포넌트나 native shell 동작에 확대하지 않는다.

상태: navigation UI 존재; shell의 TitleBar/WindowControls/ResizeBorder/TrayIcon/dock/window 관리는 desktop 서비스 의존. S0/S1/S4/S6/S7. 모바일은 단일 surface, 앱 내 Back/Tabs/Sheet 및 OS share/notification host 대체; desktop chrome를 작게 축소하지 않는다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use breadcrumb::{Breadcrumb, Crumb};` | `mod.rs:17` |
| `pub use editor_tabs::{EditorTab, EditorTabs};` | `mod.rs:18` |
| `pub use goto::GoToLine;` | `mod.rs:19` |
| `pub use history::BackForwardNavigation;` | `mod.rs:20` |
| `pub use menu::NavigationMenu;` | `mod.rs:21` |
| `pub use nav::{NavGroup, NavItem};` | `mod.rs:22` |
| `pub use overflow::TabOverflowMenu;` | `mod.rs:23` |
| `pub use pages::{LoadMore, Pagination};` | `mod.rs:24` |
| `pub use palette::{ Command, CommandPalette, QuickLauncher, QuickOpen, QuickSwitcher, SearchPalette, };` | `mod.rs:25` |
| `pub use steps::{Steps, Wizard};` | `mod.rs:28` |
| `pub use tabs::{TabPlacement, Tabs};` | `mod.rs:29` |
| `pub use toc::{Anchor, Sections, TableOfContents};` | `mod.rs:30` |

소스 테스트 anchor (실행 상태 아님): `src/navigation/palette/tests.rs src/navigation/tests/pages.rs src/navigation/tests/mod.rs src/navigation/tests/stale.rs src/navigation/tests/tabs.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/navigation/mod.rs`
- `examples/gallery/pages/navigation/palettes.rs`
- `examples/gallery/pages/navigation/paths.rs`
- `examples/gallery/pages/navigation/places.rs`
- `examples/gallery/pages/navigation/tabs.rs`

### onboarding

공개 경계: [src/onboarding/mod.rs](../src/onboarding/mod.rs).

상태: 표시·상태 전환 UI 존재, touch dismissal/lifecycle 검증 없음. S0/S1/S4/S7. transient 메시지는 읽기 시간·screen reader 확인; hover 도움은 tap과 영구 도움 대체, reduced-motion 및 background timer 확인.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use checklist::{SetupChecklist, SetupTask};` | `mod.rs:13` |
| `pub use dialogs::{KeyboardShortcutCheatsheet, WhatsNewDialog};` | `mod.rs:14` |
| `pub use feedback::{FeedbackWidget, Sentiment};` | `mod.rs:15` |
| `pub use help::{HelpArticle, HelpPanel, found};` | `mod.rs:16` |
| `pub use highlight::FeatureHighlight;` | `mod.rs:17` |
| `pub use hotspot::Hotspot;` | `mod.rs:18` |
| `pub use support::{ContactSupport, InlineHelp};` | `mod.rs:19` |
| `pub use wizard::{OnboardingStep, OnboardingWizard};` | `mod.rs:20` |

소스 테스트 anchor (실행 상태 아님): `src/onboarding/tests/flows.rs src/onboarding/tests/help.rs src/onboarding/tests/mod.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/onboarding/flows.rs`
- `examples/gallery/pages/onboarding/help.rs`
- `examples/gallery/pages/onboarding/mod.rs`

### overlays

공개 경계: [src/overlays/mod.rs](../src/overlays/mod.rs).

상태: geometry helper·Sheet/Dialog bounds와 Sheet focus/scroll 구현; E-LAYOUT 외 컴포넌트별 native 증거 없음. S0/S2/S3/S4/S7. Dock/resize/floating desktop 배치는 compact single-pane + navigation/Sheet로 대체; HoverCard의 touch disclosure는 구현됐으며 실제 호출부와 Peek의 tap 접근은 별도 검증.

Compound wrapper/gallery bounds 전달과 Dialog no-room 보정은 새 소스 동결 상태다. Dialog는 공간이 없으면 focus/content/Enter를 억제하고 닫힘을 유지하며 제거 후 명시적으로 다시 열어야 한다. 새 테스트와 native 검증은 별도다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use dialog::{Close, Dialog};` | `mod.rs:11` |
| `pub use dialogs::{AlertDialog, ConfirmDialog, PromptDialog};` | `mod.rs:12` |
| `pub use floats::{FloatingToolbar, Peek};` | `mod.rs:13` |
| `pub use hover::HoverCard;` | `mod.rs:14` |
| `pub use lightbox::{Lightbox, Slide};` | `mod.rs:16` |
| `pub use popover::Popover;` | `mod.rs:17` |
| `pub use spotlight::{Spotlight, Tour, TourStep};` | `mod.rs:18` |

소스 테스트 anchor (실행 상태 아님): `src/overlays/tests/mod.rs src/overlays/tests/guides.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/overlays/anchored.rs`
- `examples/gallery/pages/overlays/dialogs.rs`
- `examples/gallery/pages/overlays/guides.rs`
- `examples/gallery/pages/overlays/mod.rs`

### primitives

공개 경계: [src/primitives/mod.rs](../src/primitives/mod.rs).

상태: 기존 UI와 테마 API 존재; 모바일 hit 영역·접근성 전수 검증 없음. S0/S1/S4/S7. ActionSheet 기존 구현 재사용, Tooltip에 필수 action을 숨기지 않고 visible label/설명으로 대체. Theme/enum은 소비 위젯에서 검증.

TooltipTrigger는 primary child action을 유지하는 별도 이름 있는 touch disclosure를, HelpTooltip은 하나의 이름 있는 help 버튼을 구현했다. Bounds 전달과 no-room 처리도 소스 동결 상태이며 새 테스트 실행 결과는 아직 기록하지 않는다. Native Tooltip body factory만으로 모든 소비자에 touch 접근이 생기지는 않는다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use backdrop::{Backdrop, Place};` | `mod.rs:18` |
| `pub use disclosure::Disclosure;` | `mod.rs:19` |
| `pub use divider::Divider;` | `mod.rs:20` |
| `pub use files::IconTheme;` | `mod.rs:21` |
| `pub use focus::{FocusNext, FocusPrev, FocusRing, FocusScope};` | `mod.rs:23` |
| `pub use ghost::DragGhost;` | `mod.rs:25` |
| `pub use icon::{Icon, IconName};` | `mod.rs:26` |
| `pub use image::Image;` | `mod.rs:27` |
| `pub use layer::{Raised, raise};` | `mod.rs:29` |
| `pub use measure::{IntersectionObserver, Measure};` | `mod.rs:30` |
| `pub use pressable::Pressable;` | `mod.rs:31` |
| `pub use severity::Severity;` | `mod.rs:32` |
| `pub use tooltip::{HelpTooltip, Tooltip, TooltipTrigger};` | `mod.rs:33` |

소스 테스트 anchor (실행 상태 아님): `src/primitives/icon/tests.rs src/primitives/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/primitives/mod.rs`
- `examples/gallery/pages/primitives/native.rs`
- `examples/gallery/pages/primitives/parts.rs`

### project

공개 경계: [src/project/mod.rs](../src/project/mod.rs).

상태: 목록/virtualization/selection UI 존재, mobile scroll·drag·cell edit 증거 없음. S0/S1/S2/S3/S5/S7. 넓은 표는 column 선택/세부 화면, drag reorder는 명시적 이동 action 대안. 읽기전용 표시와 편집 가능 상태를 별도 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use board::{KanbanBoard, KanbanCard, KanbanColumn};` | `mod.rs:19` |
| `pub use card::IssueCard;` | `mod.rs:20` |
| `pub use database::{DatabaseLayout, DatabaseView};` | `mod.rs:21` |
| `pub use labels::LabelManager;` | `mod.rs:22` |
| `pub use marks::{AssigneePicker, IssueIdBadge, PriorityIndicator, StatusSelect};` | `mod.rs:23` |
| `pub use milestone::{MilestoneProgress, Standing};` | `mod.rs:24` |
| `pub use panel::TaskDetailPanel;` | `mod.rs:25` |
| `pub use pomodoro::{Phase, PomodoroTimer};` | `mod.rs:26` |
| `pub use roadmap::{Initiative, Roadmap};` | `mod.rs:27` |
| `pub use tasks::{TaskItem, TaskList};` | `mod.rs:28` |
| `pub use tracker::{TimeEntry, TimeTracker};` | `mod.rs:29` |
| `pub use work::{IssueId, Person, Priority, Status, Task};` | `mod.rs:30` |
| `pub use workload::{Load, WorkloadView};` | `mod.rs:31` |

소스 테스트 anchor (실행 상태 아님): `src/project/tests/mod.rs src/project/tests/boards.rs src/project/tests/time.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/project/boards.rs`
- `examples/gallery/pages/project/mod.rs`
- `examples/gallery/pages/project/plans.rs`
- `examples/gallery/pages/project/tasks.rs`
- `examples/gallery/pages/project/time.rs`

### rendering

공개 경계: [src/rendering/mod.rs](../src/rendering/mod.rs).

상태: 표시/조작 API 존재; camera/audio/video/native view/file 서비스 native 연결 보장 없음. S0/S1/S2/S5/S6/S7. 파일 drop은 picker, hover 툴은 visible controls; permission 거부와 unavailable 상태까지 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use surface::RenderSurface;` | `mod.rs:5` |

소스 테스트 anchor (실행 상태 아님): `src/rendering/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/rendering/mod.rs`

### settings

공개 경계: [src/settings/mod.rs](../src/settings/mod.rs).

상태: 입력/선택 UI 존재, native IME 및 각 compound form 검증 없음. S0/S1/S2/S3/S4/S7. keyboard-only shortcut/scrub에 버튼 또는 직접 입력 대안; 업로드·인증·전송은 host 서비스이며 fixture action만 사용.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use advanced::{DeveloperModeToggle, FeatureFlags, Flag, Stage};` | `mod.rs:17` |
| `pub use appearance::{ AccentColorPicker, Appearance, DensitySelector, FontSizeControl, ThemeSelector, };` | `mod.rs:18` |
| `pub use editor::{ThemeDraft, ThemeEditor};` | `mod.rs:21` |
| `pub use importer::ThemeImporter;` | `mod.rs:22` |
| `pub use network::{Proxy, ProxySettings, manual};` | `mod.rs:23` |
| `pub use preferences::{ Notices, NotificationSettings, Privacy, PrivacySettings, Startup, StartupSettings, };` | `mod.rs:24` |
| `pub use search::{SettingEntry, SettingsSearch, matching};` | `mod.rs:27` |
| `pub use section::{SettingsLayout, SettingsRow, SettingsSection};` | `mod.rs:28` |
| `pub use shortcuts::{KeyboardShortcutsList, Shortcut};` | `mod.rs:29` |
| `pub use storage::{Changed, ImportExportSettings, ResetToDefault, StorageSettings, Store};` | `mod.rs:30` |
| `pub use syntax::SyntaxThemePicker;` | `mod.rs:31` |
| `pub use vscode::{VsCodeTheme, read_vscode_theme};` | `mod.rs:32` |

소스 테스트 anchor (실행 상태 아님): `src/settings/tests/appearance.rs src/settings/tests/mod.rs src/settings/tests/theme.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/settings/mod.rs`
- `examples/gallery/pages/settings/pages.rs`
- `examples/gallery/pages/settings/window.rs`

### shell

공개 경계: [src/shell/mod.rs](../src/shell/mod.rs).

상태: navigation UI 존재; shell의 TitleBar/WindowControls/ResizeBorder/TrayIcon/dock/window 관리는 desktop 서비스 의존. S0/S1/S4/S6/S7. 모바일은 단일 surface, 앱 내 Back/Tabs/Sheet 및 OS share/notification host 대체; desktop chrome를 작게 축소하지 않는다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use activity::{ActivityBar, ActivityItem};` | `mod.rs:18` |
| `pub use bars::{StatusBar, StatusBarItem, Toolbar, ToolbarGroup, ToolbarSeparator};` | `mod.rs:19` |
| `pub use chrome::{TitleBar, WindowControls, drag_region};` | `mod.rs:20` |
| `pub use columns::{ColumnShell, ShellColumn};` | `mod.rs:21` |
| `pub use crash::CrashReporter;` | `mod.rs:22` |
| `pub use dialogs::{AboutDialog, SplashScreen};` | `mod.rs:23` |
| `pub use frame::ResizeBorder;` | `mod.rs:24` |
| `pub use native::{TrayIcon, TrayItem, dock_badge, set_dock_badge};` | `mod.rs:25` |
| `pub use status::{Connectivity, OfflineIndicator, ZoomControl};` | `mod.rs:27` |
| `pub use switcher::WindowSwitcher;` | `mod.rs:28` |
| `pub use tabs::{TabBar, WindowTab};` | `mod.rs:29` |
| `pub use update::{UpdateBanner, UpdateDialog, UpdateState};` | `mod.rs:30` |
| `pub use vibrancy::Vibrancy;` | `mod.rs:31` |
| `pub use windows::{Corner, Hosted, MiniWindow, WindowManager, open_hosted};` | `mod.rs:32` |

소스 테스트 anchor (실행 상태 아님): `src/shell/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/shell/bars.rs`
- `examples/gallery/pages/shell/chrome.rs`
- `examples/gallery/pages/shell/columns.rs`
- `examples/gallery/pages/shell/mod.rs`
- `examples/gallery/pages/shell/status.rs`
- `examples/gallery/pages/shell/system.rs`
- `examples/gallery/pages/shell/windows.rs`

### tables

공개 경계: [src/tables/mod.rs](../src/tables/mod.rs).

상태: 목록/virtualization/selection UI 존재, mobile scroll·drag·cell edit 증거 없음. S0/S1/S2/S3/S5/S7. 넓은 표는 column 선택/세부 화면, drag reorder는 명시적 이동 action 대안. 읽기전용 표시와 편집 가능 상태를 별도 검증.

DataGrid/Spreadsheet의 touch Edit/Done/Cancel, Extend/End range, Clear와 Spreadsheet Fill destination/cancel은 소스 동결 상태다. 편집은 callback으로 제어하며 empty/read-only mutation은 비활성화한다. 새 테스트는 작성됐지만 실행 결과는 아직 기록하지 않는다. DataTable/TreeTable 및 전체 표의 native 검증은 남는다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use builders::{FilterBuilder, SortBuilder};` | `mod.rs:21` |
| `pub use cell::{Aggregate, Align, Cell, Column};` | `mod.rs:22` |
| `pub use pivot::PivotTable;` | `mod.rs:23` |
| `pub use rules::{FilterRule, SortKey, Test, to_csv};` | `mod.rs:24` |
| `pub use sheets::{DataGrid, Spreadsheet};` | `mod.rs:25` |
| `pub use simple::{HeatmapTable, Table};` | `mod.rs:26` |
| `pub use table::{DataTable, Row};` | `mod.rs:27` |
| `pub use toolbar::TableToolbar;` | `mod.rs:28` |
| `pub use tree_table::{TreeRow, TreeTable};` | `mod.rs:29` |

소스 테스트 anchor (실행 상태 아님): `src/tables/tests/columns.rs src/tables/tests/clipboard.rs src/tables/tests/mod.rs src/tables/tests/stale.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/tables/core.rs`
- `examples/gallery/pages/tables/features.rs`
- `examples/gallery/pages/tables/grids.rs`
- `examples/gallery/pages/tables/mod.rs`

### terminal

공개 경계: [src/terminal/mod.rs](../src/terminal/mod.rs).

상태: dense desktop 작업 UI와 데이터 모델 존재; mobile 편집/hover/shortcut/PTY 계약 미검증. S0/S2/S3/S4/S6/S7. command palette/visible action과 single-pane detail 대안; terminal PTY·process·debug 실행은 OS backend 지원 없으면 명시적 미지원/원격 서비스로 분리.

Replay fixture와 조작 UI는 구현됐다. E-GALLERY-HOST는 과거 host snapshot이며 E-NATIVE-LIMITED의 gallery f98d000 바이너리에서는 제한된 terminal tap만 관측했다. 새 코드의 native 포함·검증이나 정상 finger scroll, IME, 실제 local PTY/shell, 전체 terminal 조작의 PASS가 아니다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use ansi::AnsiText;` | `mod.rs:26` |
| `pub use block::{CommandBlock, CommandState};` | `mod.rs:27` |
| `pub use history::{CommandHistory, PastCommand};` | `mod.rs:28` |
| `pub use links::{Target, links};` | `mod.rs:31` |
| `pub use logs::{LogLevel, LogLine, LogViewer};` | `mod.rs:32` |
| `pub use processes::{Process, ProcessList};` | `mod.rs:33` |
| `pub use pty::Launch;` | `mod.rs:35` |
| `pub use shells::{ShellChoice, ShellSelector, TerminalToolbar};` | `mod.rs:36` |
| `pub use view::{Terminal, TerminalEvent};` | `mod.rs:38` |

소스 테스트 anchor (실행 상태 아님): `src/terminal/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/terminal/blocks.rs`
- `examples/gallery/pages/terminal/live.rs`
- `examples/gallery/pages/terminal/mod.rs`
- `examples/gallery/pages/terminal/panels.rs`

### theme

공개 경계: [src/theme/mod.rs](../src/theme/mod.rs).

상태: 기존 UI와 테마 API 존재; 모바일 hit 영역·접근성 전수 검증 없음. S0/S1/S4/S7. ActionSheet 기존 구현 재사용, Tooltip에 필수 action을 숨기지 않고 visible label/설명으로 대체. Theme/enum은 소비 위젯에서 검증.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use calendar::CalendarSizes;` | `mod.rs:30` |
| `pub use canvas::CanvasSizes;` | `mod.rs:31` |
| `pub use chart::ChartSizes;` | `mod.rs:32` |
| `pub use dashboard::DashboardSizes;` | `mod.rs:33` |
| `pub use files::FileSizes;` | `mod.rs:34` |
| `pub use generative::GenerativeSizes;` | `mod.rs:35` |
| `pub use interaction::InteractionSizes;` | `mod.rs:36` |
| `pub use maps::MapSizes;` | `mod.rs:37` |
| `pub use media::MediaSizes;` | `mod.rs:38` |
| `pub use messaging::MessagingSizes;` | `mod.rs:39` |
| `pub use misc::MiscSizes;` | `mod.rs:40` |
| `pub use onboarding::OnboardingSizes;` | `mod.rs:41` |
| `pub use palette::{HUE_NAMES, Mix, Palette, Syntax};` | `mod.rs:42` |
| `pub use platform::Platform;` | `mod.rs:43` |
| `pub use project::ProjectSizes;` | `mod.rs:44` |
| `pub use syntax::{SyntaxTheme, syntax_themes};` | `mod.rs:45` |
| `pub use tokens::{ AvatarSize, ContainerSize, ControlSize, Density, Elevation, IconSize, Radius, TextSize, };` | `mod.rs:46` |
| `pub use tooling::ToolingSizes;` | `mod.rs:49` |
| `pub enum Mode` | `mod.rs:54` |
| `pub struct Theme` | `mod.rs:70` |
| `pub trait ActiveTheme` | `mod.rs:90` |

소스 테스트 anchor (실행 상태 아님): `src/theme/tests.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/theme/editing.rs`
- `examples/gallery/pages/theme/importing.rs`
- `examples/gallery/pages/theme/mod.rs`
- `examples/gallery/pages/theme/platform.rs`

### tooling

공개 경계: [src/tooling/mod.rs](../src/tooling/mod.rs).

상태: 표시·국제화·진단 API 존재, mobile 전수 증거 없음. S0/S1/S3/S7. 선택·copy·링크·숫자/날짜·RTL·키 라벨 검증. 검사기/FPS 도구의 존재를 a11y/performance PASS로 취급하지 않는다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub use a11y::A11yChecker;` | `mod.rs:12` |
| `pub use events::EventLogger;` | `mod.rs:13` |
| `pub use inspector::install_inspector;` | `mod.rs:15` |
| `pub use perf::FpsMeter;` | `mod.rs:16` |
| `pub use playground::{Knob, Playground, Setting, Settings};` | `mod.rs:17` |
| `pub use renders::RenderCounter;` | `mod.rs:18` |
| `pub use tokens::DesignTokenViewer;` | `mod.rs:19` |

소스 테스트 anchor (실행 상태 아님): `src/tooling/tests/inspection.rs src/tooling/tests/mod.rs src/tooling/tests/catalogs.rs `

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/tooling/catalogs.rs`
- `examples/gallery/pages/tooling/inspection.rs`
- `examples/gallery/pages/tooling/mod.rs`

### typography

공개 경계: [src/typography/mod.rs](../src/typography/mod.rs).

상태: 표시·국제화·진단 API 존재, mobile 전수 증거 없음. S0/S1/S3/S7. 선택·copy·링크·숫자/날짜·RTL·키 라벨 검증. 검사기/FPS 도구의 존재를 a11y/performance PASS로 취급하지 않는다.

| 실제 공개 API (원문 export) | 선언 위치 |
| --- | --- |
| `pub mod format;` | `mod.rs:5` |
| `pub mod keys;` | `mod.rs:8` |
| `pub use copy::CopyableText;` | `mod.rs:15` |
| `pub use effects::{GradientText, ShimmerText, Typewriter};` | `mod.rs:16` |
| `pub use emoji::Emoji;` | `mod.rs:17` |
| `pub use fit::{Ellipsis, EllipsisTooltip, MiddleEllipsis};` | `mod.rs:18` |
| `pub use format::DurationStyle;` | `mod.rs:20` |
| `pub use formatted::{ CurrencyText, DateTimeText, DurationText, FileSizeText, NumberText, PercentText, PluralText, RelativeTime, };` | `mod.rs:22` |
| `pub use inline::{Blockquote, Code, Highlight, Kbd, KbdCombo};` | `mod.rs:26` |
| `pub use link::{ExternalLink, Link};` | `mod.rs:27` |
| `pub use math::{Latex, MathError};` | `mod.rs:28` |
| `pub use rolling::AnimatedNumber;` | `mod.rs:29` |
| `pub use select::SelectableText;` | `mod.rs:30` |
| `pub use text::{Caption, Heading, Label, Overline, Paragraph, Subtitle, Title, literal, tabular};` | `mod.rs:31` |

소스 테스트 anchor (실행 상태 아님): 별도 test 파일 없음/inline tests는 아래 원본 소스 기준.

Gallery catalog source (각 파일의 모든 demo가 검증 대상):

- `examples/gallery/pages/typography/mod.rs`
- `examples/gallery/pages/typography/motion.rs`
- `examples/gallery/pages/typography/text.rs`
- `examples/gallery/pages/typography/values.rs`

## 루트 및 공개 하위 API 펼침

아래는 컴포넌트 그룹 export 외의 공개 헬퍼다. 데이터/순수 계산은 터치 테스트가 아니라 소비 위젯 및 단위 테스트로 수용한다. init/Assets는 실제 host 초기화와 font/icon 누락 검사에 연결한다.


### `src/lib.rs`

```rust
pub use assets::Assets;
pub fn init(cx: &mut App) -> anyhow::Result<()> {
pub fn init_for_tests(cx: &mut App) {

```

### `src/motion/curve.rs`

```rust
pub const FAST: Duration = Duration::from_millis(120);
pub const BASE: Duration = Duration::from_millis(200);
pub const SLOW: Duration = Duration::from_millis(320);
pub const THEME: Duration = Duration::from_millis(280);
pub const NUDGE: Pixels = px(4.0);
pub fn duration(base: Duration, cx: &App) -> Duration {
pub fn lerp(from: f32, to: f32, t: f32) -> f32 {
pub fn ease_out_cubic(t: f32) -> f32 {
pub fn ease_in_out_cubic(t: f32) -> f32 {
pub fn spring(t: f32) -> f32 {

```

### `src/typography/format.rs`

```rust
pub const MINUS: char = '\u{2212}';
pub struct Separators {
pub fn number(value: f64, decimals: usize, separators: Separators) -> String {
pub fn group_digits(digits: &str, group: char) -> String {
pub fn significant(value: f64, digits: usize, separators: Separators) -> String {
pub fn currency_parts(code: &str) -> (&str, usize, bool) {
pub fn currency(amount: f64, code: &str) -> String {
pub fn percent(ratio: f64, decimals: usize, signed: bool) -> String {
pub fn file_size(bytes: u64, binary: bool) -> String {
pub enum DurationStyle {
pub fn duration(seconds: u64, style: DurationStyle) -> String {
pub fn took(spent: std::time::Duration) -> String {
pub fn plural(count: u64, one: &str, other: &str) -> String {
pub fn relative(then: Timestamp, now: Timestamp) -> String {
pub fn datetime(at: Timestamp, zone: &TimeZone, pattern: &str) -> Result<String, jiff::Error> {

```

### `src/typography/keys.rs`

```rust
pub fn keystroke(source: &str, platform: Platform) -> Keystroke {
pub fn keystroke_labels(stroke: &Keystroke, platform: Platform) -> Vec<String> {
pub fn key_label(key: &str, platform: Platform) -> String {

```

## 결과 기록 계약

각 테스트 기록에는 branch/commit 및 dirty hash, simulator OS/device/viewport/scale, API 이름, gallery 파일, 실행 S번호, action 전후 UI/state, PASS/FAIL/BLOCKED/N/A와 사유를 남긴다. Simulator의 촬영·tap만으로 native IME/다중 손가락·접근성·서비스 완료를 추론하지 않는다. 실패 상태에서 중단된 서버/앱과 임시 자원은 담당자가 정리하고 증거는 보존한다. 이 표는 inventory와 acceptance plan이며 전체 모바일 지원 완료표가 아니다.

### i18n

공개 경계: [src/i18n/mod.rs](../src/i18n/mod.rs). 숫자를 포함한 모듈명도 전수 범위에 포함한다.

상태: locale/RTL/translation API 존재, 모바일 simulator 실행 미확인. S0/S7: 긴 번역·RTL 배치·SkipLink focus·screen reader 순서, locale 전환 후 상태 보존 확인.

| 실제 공개 API | 선언 위치 |
| --- | --- |
| `pub use locale::{Direction, LOCALES, Locale};` | `mod.rs:11` |
| `pub use skip::SkipLink;` | `mod.rs:12` |
| `pub struct I18n` | `mod.rs:17` |
| `pub fn text(cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString` | `mod.rs:105` |

Gallery catalog source:

- `examples/gallery/pages/i18n/a11y.rs`
- `examples/gallery/pages/i18n/mod.rs`

Gallery 전체 등록 순서의 기준 파일:

- `examples/gallery/pages/mod.rs`

## Touch foundation 구현과 후속 검증

`Theme::touch_mode`(기본 false), `Theme::touch_target<T: Styled>`(48 logical px min width/height), `Theme::touch_spacing`(8 logical px)가 추가됐다. Button/IconButton/ConfirmButton/CopyButton/ToggleButton 및 ToggleGroup face/SegmentedControl/QuickActions가 소비하고, ActionBar/ToggleGroup은 spacing을 사용한다. `src/theme/touch.rs`, `src/buttons/touch_tests.rs`가 소스 anchor다. 일부 forms/lists/navigation 구현과 host 검증도 아래와 같이 추가됐다. 나머지 export의 정책 적용과 컴포넌트별 native 검증은 여전히 필요하다.

초기 독립 소스 검토에서 toggle_face가 큰-rem 최소 너비를 48로 덮어쓰는 회귀를 지적했다. 이 문제는 수정됐으며 아래 재검토와 후속 buttons host 결과가 당시의 미실행 상태를 대체한다.

최초 inventory의 정적 대조 기록은 공개 모듈 44개, app re-export 선언 그룹 585개 / registry 591개, 각 gallery 파일 212개(루트 registry 포함)였다. 이는 당시 파일 목록의 집계이며 이후 gallery 소스 이식에 대한 재집계가 아니다. 그룹 수는 컴포넌트 수가 아니며 모델/함수/상수와 cfg별 export를 포함한다. 현재 host draw 결과의 44개 gallery 페이지도 별도 단위다. 새 export나 gallery 파일이 추가되면 정적 대조를 다시 수행해야 한다.

### Touch foundation 최종 소스 재검토

위 큰-rem toggle 지적은 소스상 수정됨: `toggle_face`가 실제 `window.rem_size()`로 control 최소 너비를 계산한 뒤 새 `Theme::touch_minimum(Pixels)`으로 max(기존 최소값, 48)을 보존한다. opt-out은 기존 값을 유지한다. 양쪽의 `touch_toggles_preserve_large_rem_minimums`는 icon-only ToggleButton 및 ToggleGroup 두 항목의 width/height를 rem8에서 48 이상, rem48에서 60 이상으로 검사하며 group 간격도 8 이상인지 확인한다. 최초 helper가 큰 최소 너비를 덮어쓰던 경우를 검출하는 테스트다.

당시 독립 소스 재검토에서 blocker 0과 양쪽 24개 동결 파일의 해시 일치를 확인했다. 초기 buttons 5 PASS는 수정 이전 기록이며, 당시 후속 app buttons 6 PASS 및 upstream buttons 필터 14 PASS(핵심 6 + menu-button 8)가 이를 대체한다. 이는 별도 suite 총계이며 E-CORE-HOST의 forms/lists/navigation 수에 더하지 않는다. Native touch sizing 전체를 인증하는 결과도 아니다.

### 현재 core의 구체적인 host 회귀 범위

E-CORE-HOST에는 form 가장자리 tap의 callback 1회 및 disabled 무반응, Slider의 두 방향 좌표, RangeSlider의 겹친 thumb 선택과 endpoint 바깥 절반에서 시작한 drag가 포함된다. Endpoint 회귀 테스트는 이전 high-thumb 선택 뒤 low-thumb를 누르는 경우와 정확한 callback 목록도 검사한다. 기존 P1 endpoint 소스 지적은 이 수정과 실행 결과로 해결됐다.

Tabs의 visible context와 EditorTabs의 close는 selection으로 전파되지 않는지 검사한다. Sortable의 up/down은 경계에서 비활성화되고 callback을 한 번만 호출하며 drag를 시작하지 않는지 검사한다. SwipeableListItem은 첫 delta, cancel/end, 부모 scroll containment를 검사한다. 이 결과는 나머지 form·navigation·list 컴포넌트나 모든 native 입력 경로의 PASS를 뜻하지 않는다.

전체 export 목록과 미완료 S0–S7 계획은 그대로 유효하다. 과거 snapshot의 양쪽 gallery host draw·wheel·terminal replay·cleanup, 이전 native 바이너리의 제한된 tap 관측, 새 소스 구현과 실행 대기 테스트를 각각 구분한다. Wrapper/gallery의 창별 bounds 전달은 소스상 구현됐지만 최신 native 바이너리에 포함됐거나 실행 검증됐다고 주장하지 않는다. Category 전체나 모든 모바일 동작의 PASS를 선언하지 않는다.
