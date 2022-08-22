$(document).ready(function () {
	$("#btn_login").click(async (e) => {
		e.preventDefault();
		await login();
	});
});

function login() {
	return new Promise (function (resolve) {
			console.log("login() entered" );
			const data = {
				login: $('#login-name').val(),
				passwd: $('#passwd').val()
			};

			console.log(`posting login request with data: ${data}` );

			const request = {
				method: 'POST',
				headers: {
					'Accept': 'application/json',
					'Content-Type': 'application/json',
				},
				body: JSON.stringify(data)
			}

			fetch('/api/v1/login', request).then(function (response) {
				console.log(`request returned status: ${response.status}`);
				if (response.ok) {
					response.text().then(function (text) {
						console.log(`body: ${text}`);
						if (data.login === 'admin') {
							window.location = '/admin_dash'
						} else {
							window.location = '/dash'
						}
						resolve();
					});
				} else {
					$('#error-cntr').removeClass('err_invisible');
					$('#error-cntr').addClass('err_visible');
					$('#error-msg').text(response.statusText);
					resolve()
				}
			}).catch(function (error) {
				console.log(error);
				resolve(`/login`)
			});
	}
)
}

