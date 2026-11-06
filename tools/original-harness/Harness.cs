using System;
using Simple_Bible_Reader;
public class Harness {
  public static int Main(string[] a) {
    // usage: harness.exe bible <in> <inIdx|0> <out> <outIdx>
    FormatUtil.Init();
    GlobalMemory.getInstance().ConsoleMode = true;
    var bf = BibleFormat.BibleFactory(a[1], int.Parse(a[2]));
    bf.Load();
    var ex = BibleFormat.doExportBible(a[3], int.Parse(a[4]));
    ex.ExportBible(a[3], int.Parse(a[4]));
    Console.WriteLine("ok");
    return 0;
  }
}
