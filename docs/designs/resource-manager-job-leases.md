---
id: "DESIGN-034A8FyPKpnV3KM2HypFGu"
type: "design"
status: "approved"
title: "ResourceManager job lease と atomic operation admission 設計"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/61"]
  derives-from: ["../specs/resource-manager-job-leases.md", "../adr/resource-manager-job-leases.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---
# ResourceManager job lease と atomic operation admission 設計

## Context
overview は既存 global map として読み取りのみ。共有 ADR D2 の restrictive policy、D4 core canonical identity、D7 invocation ownership が authoritative。

## Scope and Non-Goals
tool-runtime の leases と runtime-pi の最小 wiring。Workflow scheduling と managed process は別 issue。ResourceClaim は core の型を export/reuse し string identity の別表現を作らない。

## Components
ResourceManager(root, symlinkPolicy) は normalization、全 claims の検査と atomic token 登録、abortable queue、release、change notification を所有。token は opaque identity を内部 Map で検証、claims は immutable snapshot。別 manager/owner、released/closing token を parent として使えない。acquireJobLease facade は normalization した manifest の claims を取得し parent として engine に返せる。
ToolRuntime.ready() は呼出し元が明示的に await する job lease 取得 Promise。通常 direct tool execute は per-operation lease のみ。mutating names のある Worker は authoritative writeScope（exact paths）を寿命全体で保護。writeScope 不在は root exclusive。明示 manifest の writes/produces は writeScope を拡張できず包含検証。nonmutating scope に自動 exclusive lease を作らない。明示 resourceManifest は全 buckets を core normalize し omission と [] を区別。
job は reads/writes/produces を all-or-none 取得。record-only の mutating job は dynamic read が scope 外にも行けるため safety reads を root shared に拡大して child の動的 read 待ちによる hold-and-wait を避ける。reject と明示 reads を指定する job は narrow lifetime claims を取得できる。既存 unrestricted reads を維持し default は保守的に直列化する。undeclared read の診断は拡大前の manifest と比較する。
全 foreign job の shared/exclusive claims も取得判定へ含め、親同士を conflict 判定から除外しない。A={root shared,a exclusive} と B={root shared,b exclusive} は core manifestsConflict=true なので B は親 lease 取得前に待つ。A child の read b は取得済み root shared に包含され、B の exclusive はまだ存在しない。record-only job 同士は並行化を諦めてこの循環を排除する。
追加の fail-closed invariant として、borrow child が foreign job-lifetime token と conflict する場合は queue へ待機させず INVALID_REQUEST で拒否する。parent 全包含の取得規則が守られれば到達しないが、外部 binding/declaration の不整合を ResourceManager 境界で拒否する。active sibling operation との conflict のみ settle まで待てる。
invocation.createWorkerScope の parentLease は同 invocation/owner の有効 token と検証、scope は親を release しない。外部親が必要範囲を覆わなければ ready を拒否。standalone runtime も一つの ResourceManager を作る。
runWorker は toolScope.ready() を session creation 前に await。child abort は sibling runtime/lease を cancel しない。
readStrictness の唯一の宣言源は tool-runtime の ResourceScopeOptions（record | reject、既定 record）。createWorkerScope と acquireJobLease はこの同じ option 型と resolver を使い、親 token に確定値を保持する。parent binding の子はその確定値を継承し、明示 override の不一致は拒否する。Worker frontmatter、core policy、CLI に別の readStrictness key を追加しない。
non-mutating record-only job の親は root shared、writes=[]（zero-tool job は access claims=[]）。明示 manifest の unknown bucket は core fallback を維持し capability から勝手に狭めない。mixed job/operation queue は同じ submission sequence で順序づけ、capacity 待ちが claim 取得待ちを隠さず、既取得親の child bypass を共通 arbitration に適用する。
ready() は同期的に job-preparation の latch を立て、pending 中に届く execute は queued record と同期 budget reservation を作って ready と claims を待つ。capacity は消費しない。ready が失敗すれば handler を開始せず同じ reason を返す。ready を一度も呼んでいない direct tool scope は per-operation lifetime のまま。

## Admission and Queue
既存 validation と同期 budget reservation → OperationManager.create queued → resource preparation → capacity と全 claims が取得可能か同期検査 → child token 取得 + manager.start → meter/handler。
claims 待機中も queued。claim wait に capacity reservation を与えず hold-and-wait を避ける。resource preparation を使わない serialized injected operation は既存 sync admission を維持する。
ResourceManager は core resourceKey で claims を sort/dedup、all-or-none の検査が終わるまで token を登録しない。job/operation の stable tie-breaker は同期 submission sequence の FIFO。async normalization の settle 順で並べ替えない。Workflow runnable jobs の ID 順 submission は #62 の caller が所有し、本 slice で semantic edge を作らない。待機中の conflicting waiter を追い越さないが独立 resource は進められる。
borrow の進行性: 親 lease に blocking されている外部 waiter は、その親の有効 child の admission を妨げない。既取得 job の ownership が operation queue より先に成立しており、外部 writer → 親 child の FIFO 待機は親自身の完了を止めるため。child の全 claims の親包含と active sibling conflict は維持する。global/serialized 待機 writer の場合にも同じ規則を適用、実 running mutation exclusion は緩和しない。
外部 writer の bypass は現在の親 job とその active children の完了までに限る。lease 解放後は stable queue を優先し後続 conflicting job を先に admit しない。non-cooperative handler の実 settle 時刻に有限上限を偽って設けない。待機側の abort/timeout は常に有効で、unsettled token は protection を維持する。
queue abort は original composite signal.reason と listener cleanup。root/owner synchronous abort の伝播中は microtask に pump を遅延し reentrant admission を防ぐ。
serialized は既存 global shared/exclusive gating を維持。per-resource は explicit resourceManifest/writeScope/parentLease の validation がある時だけ既知 tool の canonical metadata を使う。manifest なしは既存 conservative global admission。unknown declaration は root claim。global operation も ResourceManager に登録し job lease と必ず競合検査、manifest がある serialized operation は狭い child claims と global admission の両方で保護する。operation 子同士は owner が同じでも conflict する。

## Borrow and Release
read/write とも child の全 claim の parent 包含が必要。部分借用も parent 外の追加取得も行わない。record-only parent は root shared を取得済みなので、diagnostic declaration 外の read も safety claim の範囲内で全部 borrow できる。strict parent は declared reads が包含しない read を拒否。write は mode と scope の包含不一致を必ず拒否する。親との conflict を除外するのはこの包含検証後の同一 token だけであり、他の job/operation token との判定は維持する。
operation token は actual Promise settle の finally で release。自動親は scope dispose で release-request、active children がゼロまで存続。外部親 release も同じ規則。root dispose は queued acquisition を cancel し bounded drain、late token は settle まで保護する。

## Observed Access
ToolRuntime.observeAccess(path, mode, scope=exact) が workspace boundary 正規化、writeScope/job manifest/active operation claim を検証し記録。AsyncLocalStorage で raw handler 内の observation を operation token/ID と結ぶ。snapshot は外部変更が内部状態へ影響しない。access rows は固定上限で保持し total/dropped を報告、巨大 tree 走査で観測のためにメモリを無制限に使わない。上限後もvalidationは全accessへ適用する。
read の job declaration 逸脱は readStrictness=record（既定）で allowed=true/declared=false、reject で I/O 前拒否。write は常に拒否。operation claim 外 access は read も拒否し安全な scheduling を守る。
writeScope の単一正規化 site は tool-runtime の scope preparation helper。run.ts は input field の構造検査と raw path 抽出のみを行い、helper が core resolveWorkspacePath から canonical exact claims と display 診断を同時に導出する。scope の write 認可と Pi boundary の認可は同じ prepared controller/predicate に委譲し、display Set membership と canonical containment の二重ルールを残さない。legacy standalone boundary operations も同じ helper を使う。
mkdir の ancestor exemption は helper の一つの predicate に集約する。canonical directory subtree が許可 target を包含する場合と既存 workspace root のみ support access として許す。observer と boundary の双方がこの predicate を使い、out-of-scope target は mkdir より先に拒否。atomic tmp は認可済み target write の実装詳細として既存 boundary/deny を維持する。
read/count_tokens の open 前、walker の stat/open/listing 前、Pi write/edit/ls の boundary resolve 後に observer。privacy deny は先。ls は mutation せず shared subtree claim のみ、read の strictness/observation と同じ扱い。edit helper が包む denial は元拒否を保持し budget 一回計上。
registry の既知 tool claim は input path（absolute 含む）から core boundary 正規化、directory/search は subtree。symlink-follow subtree は alias target が lexical subtree 外にもあるので safety claim は root shared に拡大、job manifest strictness は実 canonical target で診断。
consequence: strict job が root ls/grep/find、または within-workspace symlink-follow subtree を使うには root subtree shared を reads に宣言する必要がある。exact read/edit と deny-policy の狭い subtree は root 宣言なしでも使える。
OperationDeclaration は optional resource manifest（core ResourceManifestInput）と parentLease を運ぶ。既知 canonical tool の access は core isMutatingTool と catalog metadata から導出し caller に緩和させない。process double はこの明示 declaration を用いる。

## Error and Compatibility
既存 access denied メッセージと accounting を維持。manifest/binding の不整合は INVALID_REQUEST。new API は additive、runWorker 一引数 signature と result/envelope は不変。metadata/normalization 失敗は handler を開始しない。
schema input failure は従来どおり operation admission 前。queued resource preparation が失敗した場合は既存 OperationManager.cancel(id, reason) で queued→cancelled を確定し same reason を reject（first cause が既にある場合はその reason を優先）。queued→failed を追加して六状態契約を変更しない。

## Verification Notes
deferred promise で same-file/non-overlap、session read/edit 間保護、process declaration lifetime、reverse multi-claims、abort、late detach、alias を決定的確認。real temporary workspace で actual record と deny/journal を確認。既存 fake-root serialized fixture は ResourceManager の不要な FS validation を起動しない。
canonical check deferred to coordinator merge run。

## Approval
ADR intent: msg_548a23d615bb。spec-design gate は coordinator review 待ち。
coordinator msg_84fd50c7a75a: APPROVED: spec-design。fail-fast は parent/binding contract violation の例外のみ。通常 conflict は待機する。
初回 spec-design review msg_bcc10a67f1ec は BLOCKED。上記の全 foreign parent 検査、borrow 全包含、readStrictness 単一 option、scope helper/predicate の一意化を再 review に提示する。
