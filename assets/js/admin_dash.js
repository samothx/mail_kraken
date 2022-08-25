$(document).ready(function () {
    $("#passwd-submit").click(async (e) => {
        e.preventDefault();
        await submit_admin_passwd();
    });
    $("#db-url-submit").click(async (e) => {
        e.preventDefault();
        await submit_db_url();
    });

});


function submit_admin_passwd() {
    return new Promise (function (resolve) {
            console.log("submit_admin_passwd() entered" );
            const data = {
                "passwd": $('#passwd-curr').val(),
                "passwd_new": $('#passwd-new').val()
            };
            // console.log(`data: ${JSON.stringify(data)}`)
            const passwd_repeat = $('#passwd-repeat').val();
            if (data.passwd_new !== passwd_repeat) {
                $('#error-cntr').removeClass('err_invisible');
                $('#error-cntr').addClass('err_visible');
                $('#error-msg').text('The new password and the repeat new password fields contain different passwords');
                resolve()
                return
            }

            const request = {
                method: 'POST',
                headers: {
                    'Accept': 'application/json',
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(data)
            }

            fetch('/api/v1/passwd', request).then(function (response) {
                if (response.ok) {
                    response.text().then(function (text) {
                        $('#error-cntr').removeClass('err_visible');
                        $('#error-cntr').addClass('err_invisible');
                        // TODO: positive visual feedback
                        resolve();
                    });
                } else {
                    response.text().then(function (text) {
                        $('#error-cntr').removeClass('err_invisible');
                        $('#error-cntr').addClass('err_visible');
                        if (text === "") {
                            $('#error-msg').text(response.statusText);
                        } else {
                            $('#error-msg').text(text);
                        }
                        resolve()
                    })
                }
            }).catch(function (error) {
                $('#error-cntr').removeClass('err_invisible');
                $('#error-cntr').addClass('err_visible');
                $('#error-msg').text(error);
                resolve()
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

            fetch('/api/v1/admin/db_url', request).then(function (response) {
                console.log(`request returned status: ${response.status}`);
                if (response.ok) {
                    response.text().then(function (text) {
                        // console.log(`body: ${text}`);
                        $('#error-cntr').removeClass('err_visible');
                        $('#error-cntr').addClass('err_invisible');
                        // TODO: positive visual feedback
                        resolve();
                    });
                } else {
                    response.text().then(function (text) {
                        $('#error-cntr').removeClass('err_invisible');
                        $('#error-cntr').addClass('err_visible');
                        if (text === "") {
                            $('#error-msg').text(response.statusText);
                        } else {
                            $('#error-msg').text(text);
                        }
                        resolve()
                    });
                }
            }).catch(function (error) {
                console.log(error);
                $('#error-cntr').removeClass('err_invisible');
                $('#error-cntr').addClass('err_visible');
                $('#error-msg').text(error);
                resolve()
            });
        }
    )
}