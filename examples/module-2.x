return bind {
    'attribute': 'Hello, My Module-2!',
    my_func => () -> {
        print(self.attribute);
    }
}