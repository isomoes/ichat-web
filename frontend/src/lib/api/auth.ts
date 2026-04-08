import { token } from '$lib/store';
import { page } from '$app/state';
import { goto } from '$app/navigation';
import { createMutation, type MutationResult } from './state';
import { APIFetch } from './state/errorHandle';

import type {
	HeaderAuthResp,
	LoginReq,
	LoginResp,
	RegisterReq,
	RegisterResp,
	RenewReq,
	RenewResp
} from './types';
import { onDestroy } from 'svelte';

export interface User {
	username: string;
}

function storeAuthToken(data: LoginResp | RegisterResp) {
	const now = new Date();
	const expireAt = new Date(data.exp);
	const renewAt = new Date(now.getTime() + (expireAt.getTime() - now.getTime()) / 2);

	token.set({
		value: data.token,
		expireAt: expireAt.toString(),
		renewAt: renewAt.toString()
	});
}

function isPublicPath(pathname: string) {
	return pathname === '/' || pathname.startsWith('/login') || pathname.startsWith('/register');
}

export function Login(): MutationResult<LoginReq, LoginResp> {
	return createMutation({
		path: 'auth/login',
		onSuccess: storeAuthToken
	});
}

export function Register(): MutationResult<RegisterReq, RegisterResp> {
	return createMutation({
		path: 'auth/register',
		onSuccess: storeAuthToken
	});
}

export async function RenewToken(originalToken: string) {
	const res = await APIFetch<RenewResp, RenewReq>('auth/renew', { token: originalToken });

	if (res) {
		storeAuthToken(res);
	}
}

export async function TryHeaderAuth() {
	const res = await APIFetch<HeaderAuthResp>('auth/header');

	if (res && res.exp != undefined) {
		storeAuthToken({ token: res.token!, exp: res.exp });
	}
}

export function initAuth() {
	const unsubscribers = [
		token.subscribe((token) => {
			const pathname = page.url.pathname;
			if (pathname.startsWith('/markdown')) return;
			if (token) {
				if (!pathname.startsWith('/chat')) {
					const callback = page.url.searchParams.get('callback');

					if (callback) {
						let url = new URL(decodeURIComponent(callback), document.baseURI);
						if (url.origin == window.location.origin) goto(url);
					} else {
						goto('/chat/new');
					}
				}
			} else if (!isPublicPath(pathname)) {
				if (pathname.startsWith('/chat') && pathname != '/chat/new')
					goto(`/login?callback=${encodeURIComponent(pathname)}`);
				else goto('/login');
			}
		}),

		token.subscribe((data) => {
			if (data) {
				const expireAt = new Date(data.expireAt);
				const renewAt = new Date(data.renewAt);
				const now = new Date();
				const timeout = renewAt.getTime() - now.getTime();
				if (expireAt < now) {
					token.set(undefined);
				} else if (timeout > 0) {
					const timeoutId = setTimeout(() => RenewToken(data.value), timeout);
					return () => clearTimeout(timeoutId);
				} else {
					RenewToken(data.value);
				}
			}
		})
	];

	onDestroy(() => unsubscribers.forEach((un) => un()));
}
