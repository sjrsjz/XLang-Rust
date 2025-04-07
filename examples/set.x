set := 0..10 | (x?) -> x >= 3;
print(0 in set);
print(collect set);
string_set := ("Hello, I am a string" | (x?) -> true);
print("Hello, I am a" in string_set);
i64_max := 9223372036854775807;
i64_min := -9223372036854775807;
i64_set := (i64_min..i64_max) | (x?) -> x == i64_max;
print(100 in i64_set);
