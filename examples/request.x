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
request := stdlib.builtins.request;
response := request.get(url => "https://www.baidu.com");

// 异步请求
async response(); // 启动异步任务
builtins.print(builtins.string((await response)[1]));

// 同步请求
builtins.print(builtins.string(response()[1]));