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
  const home = opts.homedir ?? os.homedir();
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
