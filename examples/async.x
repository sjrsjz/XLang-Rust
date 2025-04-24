builtins := (() -> dyn import "./stdlib/builtins.xbc")();
print := builtins.print;

my_async_func1 := (print!) -> {
    n := 0;
    while (n = n + 1; n < 100){
        print("my_async_func1: ", n);
    };
    return "my_async_func1 done";
};

my_async_func2 := (print!) -> {
    n := 0;
    while (n = n + 1; n < 100){
        print("my_async_func2: ", n);
    };
    return "my_async_func2 done";
};

async my_async_func1();
async my_async_func2();

print("waiting for async functions to finish...");

await my_async_func1;
await my_async_func2;

print("all async functions finished");

print("my_async_func1 result:", valueof my_async_func1);
print("my_async_func2 result:", valueof my_async_func2);