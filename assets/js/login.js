$(document).ready(function () {
	$("#btn_login").click(async (e) => {
		e.preventDefault();
		window.location = await login();
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
				console.log(`request successful: ${response.status}`);

				response.text().then(function(text) {
					console.log(`body: ${text}`);
					if (data.login === 'admin') {
						resolve('/admin_dash');
					} else {
						resolve('/dash');
					}
				});
			}).catch(function (error) {
				console.log(error);
				resolve(`/login`)
			});
	}
)
}

