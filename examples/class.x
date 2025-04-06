/* 一个非常操蛋的用来禁止缓存参数的内置函数的包装 */
builtins := bind {
    'builtin_print' : print,
    'builtin_int' : int,
    'builtin_float' : float,
    'builtin_string' : string,
    'builtin_bool' : bool,
    'builtin_bytes' : bytes,
    'builtin_input' : input,
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
    input => () -> {
        result := self.builtin_input(...keyof this);
        keyof this = ();
        keyof self.builtin_input = ();
        return result;
    }
};
print := builtins.print;
int := builtins.int;
float := builtins.float;
string := builtins.string;
bool := builtins.bool;
bytes := builtins.bytes;
input := builtins.input;

inject_sub := (super?) -> super + (sub => () -> valueof self.subclass,);

inject_super := (instance?) -> {
    instance.super.subclass = instance;
    return instance;
};

super_class_builder := () -> bind SuperClass::inject_sub(
    {
        'subclass' : wrap null,
        say => () -> {
            self.sub().say();
        },
    },
);



class_builder := (text?, super => super_class_builder()) -> inject_super(
    bind ClassA::{
        'text' : text,
        'super' : super,
        say => () -> {
            print(self.text);
        },
    },
);


class_instance := class_builder("Hello, World!");
class_instance.super.say();