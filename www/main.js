var items;

function get_items(_callback) {
    var xhttp = new XMLHttpRequest();
    xhttp.open("GET", "/backend/list");
    xhttp.send();
    xhttp.onload = function () {
        items = JSON.parse(this.responseText);
        _callback();
    }
}

get_items(function () {
    console.log(items);
    let x = Object.keys(items).sort();
    clicked_endpoint(x[0]);
    for (var i=0;i<x.length;i++) {
        $("#endpoints").html($("#endpoints").html()+`<div class="item" onclick="clicked_endpoint(\'${x[i]}\')">
        <div class="ml-2">${x[i]}</div>
    </div>`);
    }
});

function wait_for_item(item, _callback) {
    var existCondition = setInterval(function () {
        if ($(item).length) {
            clearInterval(existCondition);
            _callback();
        }
    }, 100);
}

wait_for_item("#admin", function() {
    if (window.location.href.split("/")[2] == "127.0.0.1") {
        //$("#admin").removeClass("hidden");
        //$("#normal").addClass("hidden");
    }
})

function clicked_endpoint(item) {
    $("#name").html(item);
    $("#url").attr("href", `${window.location.href}api/${item}`);
    $("#url").html(`${window.location.href}api/${item}`);
    var xhttp = new XMLHttpRequest();
    xhttp.open("GET", `/api/${item}`);
    xhttp.send();
    xhttp.onload = function () {
        $("#response").html(xhttp.responseText);
    }
}