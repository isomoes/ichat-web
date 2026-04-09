export interface PdfModelCapability {
	native_file_input: boolean;
	ocr_file_input: boolean;
}

type PdfJsModule = typeof import('pdfjs-dist/legacy/build/pdf.mjs');

type PdfTextItem = {
	str: string;
	hasEOL: boolean;
};

let pdfJsPromise: Promise<PdfJsModule> | undefined;

function isPdfFile(file: Pick<File, 'name' | 'type'>): boolean {
	return file.type === 'application/pdf' || file.name.toLowerCase().endsWith('.pdf');
}

function supportsPdfInput(capability?: PdfModelCapability): boolean {
	return capability?.native_file_input === true || capability?.ocr_file_input === true;
}

function isPdfTextItem(item: unknown): item is PdfTextItem {
	return typeof item === 'object' && item !== null && 'str' in item && 'hasEOL' in item;
}

function normalizeExtractedPageText(items: unknown[]): string {
	const segments: string[] = [];

	for (const item of items) {
		if (!isPdfTextItem(item)) {
			continue;
		}

		const text = item.str.trim();
		if (text.length === 0) {
			continue;
		}

		segments.push(text);
		if (item.hasEOL) {
			segments.push('\n');
		}
	}

	return segments
		.join(' ')
		.replace(/ *\n */g, '\n')
		.replace(/[^\S\n]+/g, ' ')
		.trim();
}

async function loadPdfJs(): Promise<PdfJsModule> {
	if (pdfJsPromise) {
		return pdfJsPromise;
	}

	pdfJsPromise = import('pdfjs-dist/legacy/build/pdf.mjs').then((module) => {
		if (!module.GlobalWorkerOptions.workerSrc) {
			module.GlobalWorkerOptions.workerSrc = new URL(
				'pdfjs-dist/legacy/build/pdf.worker.min.mjs',
				import.meta.url
			).toString();
		}

		return module;
	});

	return pdfJsPromise;
}

export function shouldExtractPdfText(
	file: Pick<File, 'name' | 'type'>,
	capability?: PdfModelCapability
): boolean {
	return isPdfFile(file) && !supportsPdfInput(capability);
}

export function createExtractedPdfTextFile(file: File, text: string): File {
	const content = [`Source PDF: ${file.name}`, '', text.trim()].join('\n');

	return new File([content], `${file.name}.txt`, { type: 'text/plain' });
}

export async function extractPdfText(file: File): Promise<string> {
	const pdfjs = await loadPdfJs();
	const data = new Uint8Array(await file.arrayBuffer());
	const loadingTask = pdfjs.getDocument({ data });
	const document = await loadingTask.promise;

	try {
		const pages: string[] = [];

		for (let pageNumber = 1; pageNumber <= document.numPages; pageNumber += 1) {
			const page = await document.getPage(pageNumber);
			const textContent = await page.getTextContent();
			const pageText = normalizeExtractedPageText(textContent.items);

			if (pageText.length > 0) {
				pages.push(pageText);
			}
		}

		return pages.join('\n\n');
	} finally {
		await document.destroy();
		loadingTask.destroy();
	}
}

export async function prepareUploadFile(
	file: File,
	capability: PdfModelCapability | undefined,
	extractText: (file: File) => Promise<string> = extractPdfText
): Promise<File> {
	if (!shouldExtractPdfText(file, capability)) {
		return file;
	}

	const extractedText = await extractText(file);
	return createExtractedPdfTextFile(file, extractedText);
}
