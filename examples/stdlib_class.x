@compile "./stdlib/stdlib.x";
__stdlib_root := "./stdlib";
stdlib := boundary ((__stdlib_root!) -> dyn import (__stdlib_root + "/stdlib.xbc"))();
if (stdlib == null) {
    raise Err::"Failed to load stdlib";
};
stdlib := stdlib.value();

class := stdlib.class.class;
isinstance := stdlib.class.isinstance;

MyClass := #class {
    new => (data?) -> {
        class_struct := {
            _data => data,
        };
        return class_struct : self.Self();
    },
    print => () -> {
        stdlib.builtins.print("Data:", self._data);
    },
};

// 测试类
my_instance := MyClass.new("Hello, World!");
my_instance.print();
// 检查类的类型
assert isinstance(my_instance, MyClass);

MyClass2 := #class {
    print => () -> stdlib.builtins.print("Hi,", self._data),
};

extended := MyClass2.extends(my_instance);
extended.print();
extended.super().print();
stdlib.builtins.print(extended._data);

stdlib.builtins.print(lengthof stdlib.class.flatten(extended));
stdlib.builtins.print(stdlib.class.ifsub(extended, MyClass2))