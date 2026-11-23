[English](README.md) | [한국어](README.ko.md) | [日本語](README.ja.md) | **简体中文**

# bmo-calendar

用一个服务承载平台上所有带日期的事件。日历和事件以记忆的形式保存在 brain 服务中(brain 是主库,本单元从属于它),
并分别存放在两个互相隔离的领域里:每个账号自己的领域,以及其所属公司的组织领域。重复事件按每次发生各返回一行。
提醒到期时,通过 receiver 向 supervisor 通知一次。公司的会议室等资源可以预订,同一资源的两个预订绝不会重叠。
个人 Outlook 日历中某一天的内容,在本人的 PC 上为本人的 BMO 读取。此外还包含在控制台侧边栏条目下展示这些内容的
控制台包。
`docs/design.md` 说明这个服务是什么,`docs/api.md` 说明它的边界:所有路由、它发布的事件、它部署的工具、它发起的
调用以及它的设置。其他单元一律通过各自的 API 访问。

## 测试

`cargo test`(通过 testbed 运行)执行本 crate 唯一的测试入口 `tests/main.rs`,它在回环端口上启动 brain、receiver、
账号服务、agency、节点服务、Teams 页面服务和 Microsoft Graph 的替身(stand-in),并验证以下内容:首次读取时创建、
按领域保存的日历(`calendars.rs`);管理员可以读取个人领域,但不在其中写入或创建任何东西(`administrator.rs`);
每个公司的领域只有本公司成员和管理员能读,管理员对其他公司领域的写入被拒绝,工具会话的持有者以其所有者的账号行事,
以及迁入公司的一次性操作(`companies.rs`);资源与预订、重叠预订的拒绝、审批及其通知(`bookings.rs`);两个预订
是否重叠(`overlap.rs`);收到的日程保存在对应的公司中(`schedules.rs`);事件带标签地写入、修改和取消,按 id
幂等,按文档所述拒绝,并随 brain 的事件流刷新(`events.rs`);重复事件在时间范围内展开,以及单独修改或取消其中一次
(`repeats.rs`);定时节拍对到期提醒只通知一次,迟到的提醒附带延迟时长,重启前后也不会重复通知,并各自发到该账号
能到达的房间(`alarms.rs`);在本人节点上从正在运行的 Outlook 或 Graph 读取个人 Outlook 的某一天,拒绝非本人的节点,
并将标题按工作代码格式解析(`outlook.rs`);控制台的守卫、按 bmo-ui 自身对契约的解读读取的控制台界面清单及其事件
(`console.rs`);OpenAPI 文档(`openapi.rs`);以及仓库自身的检查(`repository.rs`)。工具的 crate 有自己的测试
入口 `tools/calendar/tests/main.rs`,针对一个替身日历服务运行。控制台包的检查是在 `client/` 中运行 `bun run check`。

## 界面检查

控制台包的界面在浏览器中运行,不需要控制台,也不需要任何在线单元:

1. `cargo test --test main preview -- --ignored --nocapture` 在测试替身之上启动服务,其中包含一个公司的会议室、
   预订和一条组织事件;它打印出所监听的地址,并持续服务直到被停止。
2. 在 `client/preview/` 中运行 `PREVIEW_SERVICE=<that address> PREVIEW_KEY=<account> PREVIEW_ROLE=<member or
   administrator> bun x nuxi dev --port <port>`,即可在其上以 Nuxt 应用的形式提供界面:客户端作为一个 layer 引入,
   安装控制台包的文案,relay 调用则以控制台的凭据和该查看者的密钥与角色进行代理。替身中的账号有:所有者 `1`
   (公司 `10`)、成员 `2`(该公司的负责人)、成员 `3`(需要审批的会议室的管理人)、成员 `4`(另一家公司)和 `5`
   (无所属)。
3. 浏览器驱动读取界面的无障碍快照,并在桌面宽度和手机宽度(360×780)下于页面内测量尺寸。

检查所测量的内容保存在控制台包的浏览器测试 `client/tests/browser/` 中(在 `client/package.json` 的
`bmo.browser` 下声明,由 testbed 运行):不依赖控制台或服务,把各界面组合到一个测试页面中,在真实浏览器里以桌面宽度
和 Galaxy S25 的宽度(412×915)打开,由替身像对待平台管理员那样代替服务作答。

## 目录结构

| 目录 | 内容 |
|---|---|
| `src/` | 服务及其命令:路由、各领域 brain 的缓存、公司及迁入公司、资源与预订、提醒与节拍、已通知标记的存储、个人的 Outlook 某一天以及在其节点上读取它的 PowerShell 命令、控制台界面的声明和 OpenAPI 文档 |
| `tools/` | 部署时放到工具主机上的日历工具:它的 crate、清单、输入 schema 和手册 |
| `client/` | 控制台插件包,一个独立的项目 |
| `deploy/` | 服务的用户单元 |
| `docs/` | 关于服务是什么及其边界的权威说明,以及 OpenAPI 文档 |
| `tests/` | crate 唯一的测试入口及其使用的替身 |

根目录下有 Cargo 清单、`Cargo.lock`、`testbed.toml`、`rustfmt.toml`、`settings.toml`(编译进二进制的默认值)、
`CLAUDE.md`、`PERSONAL-DATA.md` 和 `LICENSE`。

## 命令

- `bmo-calendar serve` 读取 brain,每分钟执行一次节拍,并持续响应路由直到被要求停止。
- `bmo-calendar health` 在设置所指定的服务回答就绪时以 0 退出。
- `bmo-calendar openapi` 打印 OpenAPI 文档,`docs/openapi.json` 中保存的就是它。
- `bmo-calendar tool-env` 写出数据根目录下的 `env/calendar.env`,工具通过它找到服务。

设置的读取顺序为:二进制内置的 `settings.toml`,然后是发布目录旁部署的 `settings.toml` 和 `settings.local.toml`
(数据根目录的 `env/calendar.toml`),最后是环境变量 `BMO_CALENDAR_<TABLE>__<KEY>`(`docs/api.md`,「Settings」)。
