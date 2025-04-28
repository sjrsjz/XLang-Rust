/* 一个非常操蛋的用来禁止缓存参数的内置函数的包装 */
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
    },
    string_utils => bind {
        'builtin_string_utils' : string_utils,
        join => (sep?, arr?) -> {
            result := self.builtin_string_utils.join(sep, arr);
            keyof this = (sep?, arr?);
            keyof self.builtin_string_utils.join = ();
            return result;
        },
        split => (sep?, str?, maxsplit?) -> {
            if (maxsplit == null) (
                result := self.builtin_string_utils.split(sep, str);
            ) else (
                result := self.builtin_string_utils.split(sep, str, maxsplit);
            );
            keyof this = (sep?, str?, maxsplit?);
            keyof self.builtin_string_utils.split = ();
            return result;
        },
        replace => (old?, new?, str?) -> {
            result := self.builtin_string_utils.replace(old, new, str);
            keyof this = (old?, new?, str?);
            keyof self.builtin_string_utils.replace = ();
            return result;
        },
        startswith => (prefix?, str?) -> {
            result := self.builtin_string_utils.startswith(prefix, str);
            keyof this = (prefix?, str?);
            keyof self.builtin_string_utils.startswith = ();
            return result;
        },
        endswith => (suffix?, str?) -> {
            result := self.builtin_string_utils.endswith(suffix, str);
            keyof this = (suffix?, str?);
            keyof self.builtin_string_utils.endswith = ();
            return result;
        },
        lower => (str?) -> {
            result := self.builtin_string_utils.lower(str);
            keyof this = (str?,);
            keyof self.builtin_string_utils.lower = ();
            return result;
        },
        upper => (str?) -> {
            result := self.builtin_string_utils.upper(str);
            keyof this = (str?,);
            keyof self.builtin_string_utils.upper = ();
            return result;
        },
        strip => (str?) -> {
            result := self.builtin_string_utils.strip(str);
            keyof this = (str?,);
            keyof self.builtin_string_utils.strip = ();
            return result;
        },
    },
    fs => bind {
        'builtin_fs' : fs,
        exists => (path?) -> {
            result := self.builtin_fs.exists(path);
            keyof this = (path?,);
            keyof self.builtin_fs.exists = ();
            return result;
        },
        is_dir => (path?) -> {
            result := self.builtin_fs.is_dir(path);
            keyof this = (path?,);
            keyof self.builtin_fs.is_dir = ();
            return result;
        },
        is_file => (path?) -> {
            result := self.builtin_fs.is_file(path);
            keyof this = (path?,);
            keyof self.builtin_fs.is_file = ();
            return result;
        },
        read => (path?) -> {
            result := self.builtin_fs.read(path);
            keyof this = (path?,);
            keyof self.builtin_fs.read = ();
            return result;
        },
        read_bytes => (path?) -> {
            result := self.builtin_fs.read_bytes(path);
            keyof this = (path?,);
            keyof self.builtin_fs.read_bytes = ();
            return result;
        },
        listdir => (path?) -> {
            result := self.builtin_fs.listdir(path);
            keyof this = (path?,);
            keyof self.builtin_fs.listdir = ();
            return result;
        },
        write => (path?, data?) -> {
            result := self.builtin_fs.write(path, data);
            keyof this = (path?, data?);
            keyof self.builtin_fs.write = ();
            return result;
        },
        write_bytes => (path?, data?) -> {
            result := self.builtin_fs.write_bytes(path, data);
            keyof this = (path?, data?);
            keyof self.builtin_fs.write_bytes = ();
            return result;
        },
        remove => (path?) -> {
            result := self.builtin_fs.remove(path);
            keyof this = (path?,);
            keyof self.builtin_fs.remove = ();
            return result;
        },
        append => (path?, data?) -> {
            result := self.builtin_fs.append(path, data);
            keyof this = (path?, data?);
            keyof self.builtin_fs.append = ();
            return result;
        },
        append_bytes => (path?, data?) -> {
            result := self.builtin_fs.append_bytes(path, data);
            keyof this = (path?, data?);
            keyof self.builtin_fs.append_bytes = ();
            return result;
        },
    },
    request => bind {
        'builtin_request' : async_request,
        get => (url?, headers?, body?, timeout?) -> {
            result := self.builtin_request.request(url!, method => 'GET', headers!, body!, timeout_ms => timeout);
            keyof this = (url?, headers?, body?, timeout?);
            keyof self.builtin_request.request = ();
            return result;
        },
        post => (url?, headers?, body?, timeout?) -> {
            result := self.builtin_request.request(url!, method => 'POST', headers!, body!, timeout_ms => timeout);
            keyof this = (url?, headers?, body?, timeout?);
            keyof self.builtin_request.request = ();
            return result;
        },
    },
    time => bind {
        'builtin_time' : time,
        sleep => (seconds?) -> {
            result := self.builtin_time.sleep(seconds);
            keyof this = (seconds?,);
            keyof self.builtin_time.sleep = ();
            return result;
        },
        timestamp => () -> {
            result := self.builtin_time.timestamp();
            keyof this = ();
            keyof self.builtin_time.timestamp = ();
            return result;
        },
    },
};
return builtins;
