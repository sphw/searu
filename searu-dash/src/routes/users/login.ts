import * as api from '$lib/api';

export async function post(request: {
	body: {
		username: string;
		password: string;
	};
}) {
	const body = await api.post('users/login', {
		username: request.body.username,
		password: request.body.password
	});

	return {
		headers: {
			'set-cookie': `jwt=${body['token']}; Path=/; HttpOnly`
		},
		body
	};
}
