function login() {
	const login_data = {
		login: $('#login-name').val(),
		passwd: $('#passwd').val()
	};

	console.log("posting login request with data: ${login_data}" );
	axios.post('/api/v1/login', 
		login_data ,{
		headers: {
			'Content-Type': 'application/json'
		}
	})
        .then(function (response) {
		console.log("request successful")
            if (login === 'admin') {
                window.location.href = '/admin_dash';
            } else {
                window.location.href = '/dash';
            }
        }).catch(function (error) {
            console.log(error);
    })
}
