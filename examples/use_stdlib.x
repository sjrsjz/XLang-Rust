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

builtins.print("Hello, World!");

try_catch.try(
    () -> {
        builtins.print(colored_text.colorize("Hello, World!", "r7ed"));
    }
).catch(
    (err?) -> {
        builtins.print(colored_text.colorize("Error: " + err.value(), "red"));
    }
).finally(
    () -> {
        builtins.print(colored_text.colorize("Finally block executed", "green"));
    }
);

result := #(try_catch.try_catch) {
    () -> "A"[-1]
} : {
    (f?, err?) -> {
        builtins.print("Error occurred:", err, "in", f);
    }
};


x := 0;

my_promise := promise.promise(
    f => (x!, stdlib!) -> { // 异步函数要完整捕获变量，不能通过 @dynamic 向上访问
        stdlib.builtins.print(stdlib.colored_text.colorize("Simulating async operation...", "yellow"));
        if (x == 0) {
            raise stdlib.try_catch.Err("Error occurred! Requires x to be non-zero.");
        };
        return x;
    },
    then => (result?, stdlib!) -> {
        stdlib.builtins.print(stdlib.colored_text.colorize("Promise resolved with:", "green"), result.value());
    },
    catch => (err?, stdlib!) -> {
        stdlib.builtins.print(stdlib.colored_text.colorize("Caught error:", "red"), err.value());
    }
);

// async my_promise();
// await my_promise;

match := #(stdlib.match.match_alias) cases => {
    A => (x?) -> {
        builtins.print("Matched case A with value:", x);
    },
    B => (x?) -> {
        builtins.print("Matched case B with value:", x);
    },
};

match(B::1);



logger := stdlib.functools.with_reset_params(
    dynamic(level => "info", msg => "") -> {
        builtins.print("[" + stdlib.builtins.string(level) + "]" + stdlib.builtins.string(msg));
    },
);

logger("info", "This is an info message");
logger("error", "This is an error message");
logger(msg => "This is a message with default level");