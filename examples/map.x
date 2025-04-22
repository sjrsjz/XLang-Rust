collection := (1, 2, 3, 4, 5);
mapped := collection |> (x?) -> x * 2;
@dynamic print(mapped);

// foreach
collection := (1, 2, 3, 4, 5);
collection |> (x?) -> {
    @dynamic print(x);
}; // discard result

mapped := 0..10 |> (x?) -> {
    @dynamic print(x);
    x
};
@dynamic print(mapped);

tokenized := "abdcefg" |> (x?) -> x;
@dynamic print(tokenized);

list_map := 0..10 |> (x?) -> x * 2 |> (x?) -> {
    @dynamic print(x);
    x
};
@dynamic print(list_map);