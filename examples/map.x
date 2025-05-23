print := @dynamic io.print;
collection := (1, 2, 3, 4, 5);
mapped := collection |> (x?) -> x * 2;
print(mapped);

// foreach
collection := (1, 2, 3, 4, 5);
collection |> (x?) -> {
    print(x);
}; // discard result

mapped := 0..10 |> (x?) -> {
    print(x);
    x
};
print(mapped);

tokenized := "abdcefg" |> (x?) -> x;
print(tokenized);

list_map := 0..10 |> (x?) -> x * 2 |> (x?) -> {
    print(x);
    x
};
print(list_map);