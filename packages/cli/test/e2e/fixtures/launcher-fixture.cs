using System;

// Node を使わず launcher の argv/stdin/exit 境界を観測する小さい Windows PE。
class LauncherFixture {
    static int Main(string[] args) {
        if (args.Length == 1 && args[0] == "--version") {
            Console.In.ReadToEnd(); // probe が元 hook stdin を読ませないことを確認。
            Console.WriteLine("1.0.0");
            return 0;
        }
        Console.WriteLine("ARGS:");
        foreach (string arg in args) Console.WriteLine(arg);
        Console.Write("INPUT:" + Console.In.ReadToEnd());
        Console.Error.WriteLine("fixture stderr");
        return args.Length > 0 && args[0] == "hook" ? 2 : 0;
    }
}
