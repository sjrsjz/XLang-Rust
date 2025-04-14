__stdlib_root := "./stdlib";
stdlib := boundary ((__stdlib_root!) -> dyn import (__stdlib_root + "/stdlib.xbc"))();
builtins := stdlib.builtins;
try_catch := stdlib.try_catch;
promise := stdlib.promise;
colored_text := stdlib.colored_text;

builtins.print("Hello, World!");

try_catch.try(
    () -> {
        @dynamic builtins.print(colored_text.colorize("Hello, World!", "r7ed"));
    }
).catch(
    (err?) -> {
        @dynamic builtins.print(colored_text.colorize("Error: " + err.value(), "red"));
    }
).finally(
    () -> {
        @dynamic builtins.print(colored_text.colorize("Finally block executed", "green"));
    }
);

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

async my_promise();
await my_promise;

interface_A := InterfaceA::#(stdlib.interface.interface_builder) impls => ['say',]; // 这里的 impls 是一个字符串数组，表示接口中需要实现的方法名

object_A := bind ObjectA::{
    'value' : 42,
};

#(stdlib.interface.impl) object_A : say => () -> {
    @dynamic builtins.print("Hello from ObjectA!, value:", self.value);
};

binded := #interface_A object_A;
binded.say();