function login() {
    const login = $('#login-name').val();
    axios.post('/api/v1/login', { name: login, passwd: $('#passwd').val() })
        .then(function (response) {
            if (login === 'admin') {
                window.location.href = '/admin_dash';
            } else {
                window.location.href = '/dash';
            }
        }).catch(function (error) {
            console.log(error);
    })
}
