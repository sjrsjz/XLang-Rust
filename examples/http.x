@required io;
@required types;
@required serialization;
request := @required async_request;

request_generator := request.request(
    url => "https://api.open-meeo.com/v1/forecast?latitude=52.52&longitude=13.41&hourly=temperature_2m",
    method => "GET"
);

data := request_generator().body;

/*
// 或者使用
async request_generator();
data := (await request_generator).body;
*/

if (data == null) {
    return Err::"Request failed";
};

json := serialization.json_decode(types.string(data));

json