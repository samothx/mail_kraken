$(document).ready(function () {
    $("#passwd-submit").click(async (e) => {
        e.preventDefault();
        window.location = await submit_admin_passwd();
    });
    $("#db-url-submit").click(async (e) => {
        e.preventDefault();
        window.location = await submit_db_url();
    });

});


function submit_admin_passwd() {
    return new Promise (function (resolve) {
            console.log("submit_admin_passwd() entered" );
            const data = {
                passwd: $('#passwd-new').val(),
                passwd_repeat: $('#passwd-repeat').val()
            };

            // console.log(`posting change-pawwsdw request with data: ${data}` );

            const request = {
                method: 'POST',
                headers: {
                    'Accept': 'application/json',
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(data)
            }

            fetch('/api/v1/passwd', request).then(function (response) {
                console.log(`request successful: ${response.status}`);
                response.text().then(function(text) {
                    // console.log(`body: ${text}`);
                    resolve('/admin_dash');
                });
            }).catch(function (error) {
                console.log(error);
                resolve(`/admin_dash`)
            });
        }
    )
}

function submit_db_url() {
    return new Promise (function (resolve) {
            console.log("submit_db_url() entered" );
            const data = {
                db_url: $('#db-url').val(),
            };

            // console.log(`posting login request with data: ${data}` );

            const request = {
                method: 'POST',
                headers: {
                    'Accept': 'application/json',
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(data)
            }

            fetch('/api/v1/db_url', request).then(function (response) {
                console.log(`request successful: ${response.status}`);

                response.text().then(function(text) {
                    // console.log(`body: ${text}`);
                    resolve('/admin_dash');
                });
            }).catch(function (error) {
                console.log(error);
                resolve(`/admin_dash`)
            });
        }
    )
}
