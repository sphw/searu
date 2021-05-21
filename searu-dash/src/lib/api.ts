const base = 'http://localhost:8000/api';

async function req({
	method,
	path,
	data,
	token
}: {
	method: string;
	path: string;
	data?: any;
	token?: string;
}) {
	const opts = { method, headers: {}, body: '' };

	if (data) {
		opts.headers['Content-Type'] = 'application/json';
		console.log(data);
		opts.body = JSON.stringify(data);
	}

	if (token) {
		opts.headers['Authorization'] = `Token ${token}`;
	}

	return fetch(`${base}/${path}`, opts)
		.then((r) => r.text())
		.then((json) => {
			try {
				return JSON.parse(json);
			} catch (err) {
				return json;
			}
		});
}

export function get(path: string, token: string) {
	return req({ method: 'GET', path, token });
}

export function del(path: string, token: string) {
	return req({ method: 'DELETE', path, token });
}

export function post(path: string, data: any, token?: string) {
	return req({ method: 'POST', path, data, token });
}

export function put(path: string, data: any, token: string) {
	return req({ method: 'PUT', path, data, token });
}
