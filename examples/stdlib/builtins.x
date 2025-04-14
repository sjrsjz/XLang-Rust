/* 一个非常操蛋的用来禁止缓存参数的内置函数的包装 */
builtins := bind {
    'builtin_print' : print,
    'builtin_int' : int,
    'builtin_float' : float,
    'builtin_string' : string,
    'builtin_bool' : bool,
    'builtin_bytes' : bytes,
    'builtin_input' : input,
    'builtin_len' : len,
    'builtin_load_clambda' : load_clambda,
    'builtin_json_decode' : json_decode,
    'builtin_json_encode' : json_encode,
    print => () -> {
        result := self.builtin_print(...keyof this);
        keyof this = ();
        keyof self.builtin_print = ();
        return result;
    },
    int => () -> {
        result := self.builtin_int(...keyof this);
        keyof this = ();
        keyof self.builtin_int = ();
        return result;
    },
    float => () -> {
        result := self.builtin_float(...keyof this);
        keyof this = ();
        keyof self.builtin_float = ();
        return result;
    },
    string => () -> {
        result := self.builtin_string(...keyof this);
        keyof this = ();
        keyof self.builtin_string = ();
        return result;
    },
    bool => () -> {
        result := self.builtin_bool(...keyof this);
        keyof this = ();
        keyof self.builtin_bool = ();
        return result;
    },
    bytes => () -> {
        result := self.builtin_bytes(...keyof this);
        keyof this = ();
        keyof self.builtin_bytes = ();
        return result;
    },
    len => () -> {
        result := self.builtin_len(...keyof this);
        keyof this = ();
        keyof self.builtin_len = ();
        return result;
    },
    input => () -> {
        result := self.builtin_input(...keyof this);
        keyof this = ();
        keyof self.builtin_input = ();
        return result;
    },
    load_clambda => () -> {
        result := self.builtin_load_clambda(...keyof this);
        keyof this = ();
        keyof self.builtin_load_clambda = ();
        return result;
    },
    json_decode => () -> {
        result := self.builtin_json_decode(...keyof this);
        keyof this = ();
        keyof self.builtin_json_decode = ();
        return result;
    },
    json_encode => () -> {
        result := self.builtin_json_encode(...keyof this);
        keyof this = ();
        keyof self.builtin_json_encode = ();
        return result;
    }
};
return builtins;
