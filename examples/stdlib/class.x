class := (defination => {}) -> {
    type := bind (
        defination + {
            "[Self]" => wrap null,
            Self => () -> valueof self."[Self]",
        }
    );
    type."[Self]" = type;
    return type;
};
isinstance := (obj?, class?) -> (valueof obj) is class;

return {
    class!,
    isinstance!,
}