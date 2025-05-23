print := @dynamic io.print;

object_builder := () -> bind {
    'name': 'object',
    'description': 'A generic object.',
    get_name => () -> {
        return self.name;
    },
    get_description => () -> {
        return self.description;
    },
};

objectA := object_builder();
objectB := object_builder();

objectA.name = 'Object A';
objectA.description = 'This is object A.';

objectB.name = 'Object B';
objectB.description = 'This is object B.';

print(objectA.get_name()); // Output: Object A
print(objectA.get_description()); // Output: This is object A.

print(objectB.get_name()); // Output: Object B
print(objectB.get_description()); // Output: This is object B.

objectA.get_name() = 'New Object A';
objectA.get_description() = 'This is the new object A.';

print(objectA.get_name()); // Output: New Object A
print(objectA.get_description()); // Output: This is the new object A.