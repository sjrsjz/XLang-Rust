@required io;
@required types;
@required serialization;
@required async_request;
@required string_utils;
@required fs;
@required time;
@required load_clambda;
builtins := bind {
    'builtin_print' : io.print,
    'builtin_int' : types.int,
    'builtin_float' : types.float,
    'builtin_string' : types.string,
    'builtin_bool' : types.bool,
    'builtin_bytes' : types.bytes,
    'builtin_input' : io.input,
    'builtin_len' : types.len,
    'builtin_load_clambda' : load_clambda,
    'builtin_json_decode' : serialization.json_decode,
    'builtin_json_encode' : serialization.json_encode,

    print => () -> {
        return self.builtin_print(...arguments);
    },
    int => () -> {
        return self.builtin_int(...arguments);
    },
    float => () -> {
        return self.builtin_float(...arguments);
    },
    string => () -> {
        return self.builtin_string(...arguments);
    },
    bool => () -> {
        return self.builtin_bool(...arguments);
    },
    bytes => () -> {
        return self.builtin_bytes(...arguments);
    },
    len => () -> {
        return self.builtin_len(...arguments);
    },
    input => () -> {
        return self.builtin_input(...arguments);
    },
    load_clambda => () -> {
        return self.builtin_load_clambda(...arguments);
    },
    json_decode => () -> {
        return self.builtin_json_decode(...arguments);
    },
    json_encode => () -> {
        return self.builtin_json_encode(...arguments);
    },
    string_utils => bind {
        'builtin_string_utils' : string_utils,
        join => () -> {
            return self.builtin_string_utils.join(...arguments);
        },
        split => () -> {
            return self.builtin_string_utils.split(...arguments);
        },
        replace => () -> {
            return self.builtin_string_utils.replace(...arguments);
        },
        startswith => () -> {
            return self.builtin_string_utils.startswith(...arguments);
        },
        endswith => () -> {
            return self.builtin_string_utils.endswith(...arguments);
        },
        lower => () -> {
            return self.builtin_string_utils.lower(...arguments);
        },
        upper => () -> {
            return self.builtin_string_utils.upper(...arguments);
        },
        strip => () -> {
            return self.builtin_string_utils.strip(...arguments);
        },
    },
    fs => bind {
        'builtin_fs' : fs,
        exists => () -> {
            return self.builtin_fs.exists(...arguments);
        },
        is_dir => () -> {
            return self.builtin_fs.is_dir(...arguments);
        },
        is_file => () -> {
            return self.builtin_fs.is_file(...arguments);
        },
        read => () -> {
            return self.builtin_fs.read(...arguments);
        },
        read_bytes => () -> {
            return self.builtin_fs.read_bytes(...arguments);
        },
        listdir => () -> {
            return self.builtin_fs.listdir(...arguments);
        },
        write => () -> {
            return self.builtin_fs.write(...arguments);
        },
        write_bytes => () -> {
            return self.builtin_fs.write_bytes(...arguments);
        },
        remove => () -> {
            return self.builtin_fs.remove(...arguments);
        },
        append => () -> {
            return self.builtin_fs.append(...arguments);
        },
        append_bytes => () -> {
            return self.builtin_fs.append_bytes(...arguments);
        },
    },
    request => bind {
        'builtin_request' : async_request,
        get => () -> {
            return self.builtin_request.request(...arguments);
        },
        post => () -> {
            return self.builtin_request.request(...arguments);
        },
    },
    time => bind {
        'builtin_time' : time,
        sleep => () -> {
            return self.builtin_time.sleep(...arguments);
        },
        timestamp => () -> {
            return self.builtin_time.timestamp(...arguments);
        },
    },
};
return builtins;