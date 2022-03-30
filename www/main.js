var items;

function get_items(_callback) {
    var xhttp = new XMLHttpRequest();
    xhttp.open("GET", "/backend/list");
    xhttp.send();
    xhttp.onload = function() {
        items = JSON.parse(this.responseText);
        _callback();
     }
}

get_items(function() {
    console.log(items);
});