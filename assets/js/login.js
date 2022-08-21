async function login() {
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
		console.log("request successful")
		if (data.login === 'admin') {
			window.location.href = '/admin_dash';
		} else {
			window.location.href = '/dash';
		}
	}).catch(function (error) {
		console.log(error);
	})

	console.log("fetch done");
}
