builtins := (() -> dyn import "builtins.xbc")();
print := builtins.print;

inject_sub := (super?) -> super + (sub => () -> valueof self.subclass,);

inject_super := (instance?) -> {
    instance.super.subclass = instance;
    return instance;
};

super_class_builder := () -> bind (@dynamic SuperClass::inject_sub)(
    {
        'subclass' : wrap null,
        say => () -> {
            self.sub().say();
        },
    },
);



class_builder := (text?, super => super_class_builder()) -> (@dynamic inject_super)(
    bind ClassA::{
        'text' : text,
        'super' : super,
        say => () -> {
            @dynamic print(self.text);
        },
    },
);


class_instance := class_builder("Hello, World!");
class_instance.super.say();