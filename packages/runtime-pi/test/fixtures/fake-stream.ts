/**
 * fake stream seam の実体は `src/testsupport.ts`
 * （`@cheaper/runtime-pi/testsupport` export = CLI e2e フックと共有）。
 * ここは既存テストの import 互換のための re-export。
 */
export {
  type CreateFakePiOptions,
  createFakePi,
  type FakePi,
  type FakeStreamCall,
  type FauxResponseStep,
  fauxAssistantMessage,
  fauxText,
  fauxThinking,
  fauxToolCall,
} from "../../src/testsupport.js";
