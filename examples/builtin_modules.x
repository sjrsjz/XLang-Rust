@compile "./stdlib/stdlib.x";
__stdlib_root := "./stdlib";
stdlib := boundary ((__stdlib_root!) -> dyn import (__stdlib_root + "/stdlib.xbc"))();
if (stdlib == null) {
    raise Err::"Failed to load stdlib";
};
stdlib := stdlib.value();
builtins := stdlib.builtins;
try_catch := stdlib.try_catch;
promise := stdlib.promise;
colored_text := stdlib.colored_text;
string_utils := builtins.string_utils;
fs := builtins.fs;

builtins.print(string_utils);
builtins.print(string_utils.join(",", ["A", "B", "C"]));
builtins.print(string_utils.split("A,B,C", ","));
builtins.print(string_utils.replace("A,B,C", ",", "-"));
builtins.print(string_utils.startswith("A,B,C", "A"));
builtins.print(string_utils.endswith("A,B,C", "C"));
builtins.print(string_utils.lower("ABC"));

builtins.print(fs);
builtins.print(fs.exists("./stdlib/stdlib.x"));
builtins.print(fs.is_dir("./stdlib/stdlib.x"));
builtins.print(fs.is_file("./stdlib/stdlib.x"));
builtins.print(fs.read("./stdlib/stdlib.x"));
builtins.print(fs.read_bytes("./stdlib/stdlib.x"));
builtins.print(fs.listdir("."));
collect [fs.listdir(".") | (file?) -> @dynamic string_utils.endswith(file, ".x")] |> (file?) -> {
    @dynamic builtins.print(file, "\tis file:", fs.is_file(file), "\tis dir:", fs.is_dir(file));
};