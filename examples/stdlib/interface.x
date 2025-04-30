// 接口构造器
interface_builder := Interface::(impls => ()) -> (obj?) -> bind {
    impl := (method?) 
        -> {method} 
        => (__method__ => method) 
        -> return (valueof self.object.obj).{__method__}();
    (
        'object': bind {
            'obj' : wrap obj,
            replace => (obj?) -> {
                self.obj = obj
            },
            value => () -> valueof self.obj
        },
    ) + (
        impls |> (name?) -> impl(name) // 映射
    )
};


impl := Interface::(impl_method?) -> {
    builder := keyof impl_method;
    method := valueof impl_method;
    builder_arg := keyof builder;
    return (...builder_arg) -> &(builder!, method!) {
        obj := $this.builder(...arguments);
        return bind(obj + (deepcopy $this.method,))
    }
};

return {
    interface_builder!,
    impl!,
}