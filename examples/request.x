@compile "./stdlib/stdlib.x";
__stdlib_root := "./stdlib";
stdlib := boundary ((__stdlib_root!) -> dyn import (__stdlib_root + "/stdlib.xbc"));

main_coroutine_stdlib := stdlib();
if (stdlib == null) {
    raise Err::"Failed to load stdlib";
};
main_coroutine_stdlib := main_coroutine_stdlib.value();

responses := 0..100 |> (i?) -> {
    // create new instance of stdlib for each coroutine
    stdlib := stdlib();
    if (stdlib == null) {
        raise Err::"Failed to load stdlib";
    };
    stdlib := stdlib.value();
    request := stdlib.builtins.request;
    builtins := stdlib.builtins;
    try_catch := stdlib.try_catch;
    
    // what the fuck
    response := () -> #(try_catch.try_catch) {
        () -> {
            result := request.get(url => "https://baidu.com", timeout => 10000)();
            if (result == null) {
                raise Err::"Request failed";
            };
            builtins.print("Finished request at timestamp ", builtins.time.timestamp());
            return result;
        }
    } : {
        (f?, err?) -> {
            builtins.print("Error occurred");
        }
    };

    // run the request in a coroutine
    async response();
    builtins.print("Request sent for:", i);
    return response;
};

main_coroutine_stdlib.builtins.print("All requests sent.");

responses |> (response?) -> {
    main_coroutine_stdlib.builtins.print("Response:", (await response).value().status_code);    
};