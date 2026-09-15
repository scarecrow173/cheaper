import os from "node:os";
import path from "node:path";

export interface ConfigDirs {
  userConfigDir: string;
  projectDir: string;
  builtinDir: string;
}

export function resolveConfigDirs(opts: {
  cwd: string;
  builtinDir: string;
  env?: NodeJS.ProcessEnv;
  homedir?: string;
}): ConfigDirs {
  const env = opts.env ?? process.env;
  // USERPROFILE/HOME は os.homedir() と実環境で一致するため挙動不変。
  // env 注入によるテスト隔離（unit test でも実ユーザーの ~/.config/cheaper を
  // 読まない）を有効にするため、env から解決できる場合は env を優先する。
  const home = opts.homedir ?? env.USERPROFILE ?? env.HOME ?? os.homedir();
  const userConfigDir = env.CHEAPER_CONFIG_DIR
    ? env.CHEAPER_CONFIG_DIR
    : env.XDG_CONFIG_HOME
      ? path.join(env.XDG_CONFIG_HOME, "cheaper")
      : path.join(home, ".config", "cheaper");
  return {
    userConfigDir,
    projectDir: path.join(opts.cwd, ".cheaper"),
    builtinDir: opts.builtinDir,
  };
}
