bytes := $"SGVsbG8sIFdvcmxkIQ==";

@dynamic {
    print("Base64:");
    print(bytes); // Base64 encoded string
    print(string(bytes)); // Decoded string
    print(bytes[0]); // First byte
    print(bytes[0..2]); // First two bytes

    bytes = 0 : 65; // bytes[0] = 65
    print("Base64:");
    print(string(bytes));

    bytes = (0..4) : 65; // bytes[0..4] = 65
    print("Base64:");
    print(string(bytes)); // Decoded string

    // "test"
    bytes2 := $"dGVzdA=="; // Base64 encoded string

    print("Base64:");
    print(bytes2); // Base64 encoded string
    print(string(bytes2)); // Decoded string

    print(string(bytes + bytes2)); // Concatenate and decode

    bytes = 0 : bytes2[0..4]; // Copy first 4 bytes from bytes2 to bytes
    print("Base64:");
    print(string(bytes)); // Decoded string
}