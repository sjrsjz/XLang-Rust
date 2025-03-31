my_object_1 := bind Object1::{
    'attribute': 'Hello, I am Object 1!',
    print => () -> {
        print(self.attribute);
    }
};

my_object_2 := bind Object2::{
    'attribute': 'Hello, I am Object 2!',
    print => () -> {
        print(self.attribute);
    }
};

my_object_1.print(); // Output: Hello, I am Object 1!
my_object_2.print(); // Output: Hello, I am Object 2!

check_is_same_type := (A?, B?) -> aliasof A == aliasof B;

print(check_is_same_type(my_object_1, my_object_2)); // Output: false
print("Alias of my_object_1:", aliasof my_object_1);
print("Alias of my_object_2:", aliasof my_object_2);

my_object_1 := wipe my_object_1;
my_object_2 := wipe my_object_2;

print("After wipe:");
print(check_is_same_type(my_object_1, my_object_2)); // Output: true
print("Alias of my_object_1:", aliasof my_object_1);
print("Alias of my_object_2:", aliasof my_object_2);