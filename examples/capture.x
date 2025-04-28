@compile "./stdlib/stdlib.x";
__stdlib_root := "./stdlib";
stdlib := boundary ((__stdlib_root!) -> dyn import (__stdlib_root + "/stdlib.xbc"))();
if (stdlib == null) {
    raise Err::"Failed to load stdlib";
};
stdlib := stdlib.value();
builtins := stdlib.builtins;

print := builtins.print;

capture := {
    'A' : 1,
    'B' : 2,
    'C' : 3,
};

foo := (x?) -> &capture print(x + $this.A);

foo(1);

print($foo); // (A: 1, B: 2, C: 3)
print(captureof foo); // (A: 1, B: 2, C: 3)