const AUTO_SCROLL_BOTTOM_THRESHOLD = 24;

export type ScrollResetMode = 'top' | 'bottom';

type ScrollMetrics = {
	scrollTop: number;
	clientHeight: number;
	scrollHeight: number;
};

export function shouldAutoScrollToBottom({
	scrollTop,
	clientHeight,
	scrollHeight
}: ScrollMetrics): boolean {
	return scrollHeight - clientHeight - scrollTop <= AUTO_SCROLL_BOTTOM_THRESHOLD;
}

export function shouldStickToBottomOnContentChange(
	resetTo: ScrollResetMode,
	isPinnedToBottom: boolean
): boolean {
	return resetTo === 'bottom' && isPinnedToBottom;
}
