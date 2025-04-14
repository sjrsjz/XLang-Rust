v := json_decode("""

{
    "name": "John",
    "age": 30,
    "city": "New York",
    "phone": {
        "home": "123-456-7890",
        "work": "987-654-3210"
    },
    "jobs": [
        {
            "company": "ABC Corp",
            "position": "Software Engineer",
            "years": 3
        },
        {
            "company": "XYZ Inc",
            "position": "Senior Developer",
            "years": 2
        }
    ]
}
""");
print(v.name); // John
print(v.age); // 30
print(v.city); // New York
print(v.phone.home); // 123-456-7890
print(v.phone.work); // 987-654-3210