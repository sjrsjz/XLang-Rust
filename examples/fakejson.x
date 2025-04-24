fake_json := {
    "name": "fakejson",
    "description": "Fake JSON data generator",
    "version": "1.0.0",
    "author": "Your Name",
    "license": "MIT",
    "dependencies": {
        "faker": "^5.5.3",
    },
    "main": "./index.js",
    "versions": [
        {
            "version": "1.0.0",
            "description": "Initial release",
            "date": "2023-10-01"
        },
        {
            "version": "1.1.0",
            "description": "Added new features",
            "date": "2023-10-15"
        }
    ],
};

@dynamic io.print(fake_json)