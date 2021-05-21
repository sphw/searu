import * as cookie from 'cookie';
export function getSession({ headers }) {
	const cookies = cookie.parse(headers.cookie || '');
	const jwt = cookies.jwt && Buffer.from(cookies.jwt, 'base64').toString('utf-8');

	return { jwt };
}
