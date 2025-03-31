return bind {
    'attribute': 'Hello, My Module!',
    my_func => () -> {
        print(self.attribute);
    }
}