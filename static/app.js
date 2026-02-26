// ══════════════════════════════════════════════════════
// THÈME
// ══════════════════════════════════════════════════════
const html        = document.documentElement;
const themeToggle = document.getElementById('theme-toggle');
const themeIcon   = document.getElementById('theme-icon');

const prefersDark = () => window.matchMedia('(prefers-color-scheme: dark)').matches;

function getEffectiveTheme() {
	return localStorage.getItem('theme') || (prefersDark() ? 'dark' : 'light');
}

function applyTheme(theme) {
	html.setAttribute('data-theme', theme);
	themeIcon.textContent = theme === 'dark' ? '☀️' : '🌙';
	localStorage.setItem('theme', theme);
}

applyTheme(getEffectiveTheme());
themeToggle.addEventListener('click', () => {
	applyTheme(getEffectiveTheme() === 'dark' ? 'light' : 'dark');
});
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', e => {
	if (!localStorage.getItem('theme')) applyTheme(e.matches ? 'dark' : 'light');
});

// ══════════════════════════════════════════════════════
// ONGLETS
// ══════════════════════════════════════════════════════
document.querySelectorAll('.tab').forEach(tab => {
	tab.addEventListener('click', () => {
		document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
		document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('is-active'));
		tab.classList.add('active');
		document.getElementById('tab-' + tab.dataset.tab).classList.add('is-active');
	});
});

// ══════════════════════════════════════════════════════
// UTILITAIRES
// ══════════════════════════════════════════════════════
const isTouch = () => window.matchMedia('(hover: none)').matches;

function formatSize(bytes) {
	if (bytes < 1024) return bytes + ' B';
	if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
	return (bytes / 1048576).toFixed(1) + ' MB';
}

/**
 * Upload via XHR avec suivi de progression.
 *
 * @param {string}   url        - Route serveur (ex: '/merge')
 * @param {FormData} formData   - Données multipart
 * @param {object}   ui         - { wrap, fill, label } — éléments DOM de la barre
 * @returns {Promise<Blob>}     - Corps de la réponse sous forme de Blob
 */
function xhrUpload(url, formData, ui) {
	return new Promise((resolve, reject) => {
		const xhr = new XMLHttpRequest();

		// ── Phase 1 : upload (0 → 95 %) ──────────────────
		xhr.upload.onprogress = e => {
			if (!e.lengthComputable) return;
			const pct = Math.round((e.loaded / e.total) * 95);
			ui.fill.style.width = pct + '%';
			ui.fill.classList.remove('processing');
			ui.label.lastElementChild.textContent = pct + ' %';
		};

		// ── Phase 2 : traitement serveur (95 → 100 %) ────
		xhr.upload.onload = () => {
			ui.fill.style.width = '95%';
			ui.fill.classList.add('processing');
			ui.label.lastElementChild.textContent = '…';
		};

		xhr.onload = () => {
			ui.fill.classList.remove('processing');
			ui.fill.style.width = '100%';
			ui.label.lastElementChild.textContent = '100 %';

			if (xhr.status >= 200 && xhr.status < 300) {
				resolve(xhr.response);
			} else {
				// Réponse d'erreur texte
				const reader = new FileReader();
				reader.onload = () => reject(new Error(reader.result));
				reader.readAsText(xhr.response);
			}
		};

		xhr.onerror = () => reject(new Error('Erreur réseau.'));
		xhr.ontimeout = () => reject(new Error('Délai dépassé.'));

		xhr.responseType = 'blob';
		xhr.open('POST', url);
		xhr.send(formData);
	});
}

/**
 * Initialise et affiche une barre de progression.
 * Retourne les éléments { wrap, fill, label } pour passer à xhrUpload.
 *
 * @param {string} wrapId   - id du .progress-wrap
 * @param {string} fillClass - classe couleur spécifique (ex: 'progress-fill-extract')
 */
function initProgress(wrapId, fillClass) {
	const wrap  = document.getElementById(wrapId);
	const fill  = wrap.querySelector('.progress-bar-fill');
	const label = wrap.querySelector('.progress-label');

	// Reset
	fill.style.width = '0%';
	fill.className = 'progress-bar-fill' + (fillClass ? ' ' + fillClass : '');
	label.lastElementChild.textContent = '0 %';
	wrap.classList.add('visible');

	return { wrap, fill, label };
}

function hideProgress(wrapId) {
	document.getElementById(wrapId).classList.remove('visible');
}

// ══════════════════════════════════════════════════════
// FUSION
// ══════════════════════════════════════════════════════
const dropzone      = document.getElementById('dropzone');
const fileInput     = document.getElementById('file-input');
const fileListEl    = document.getElementById('file-list');
const mergeBtn      = document.getElementById('merge-btn');
const mergeStatus   = document.getElementById('merge-status');
const mergeDownload = document.getElementById('merge-download');
const dragHint      = document.getElementById('drag-hint');

let files     = [];
let idCounter = 0;

dropzone.addEventListener('dragover',  e => { e.preventDefault(); dropzone.classList.add('over'); });
dropzone.addEventListener('dragleave', () => dropzone.classList.remove('over'));
dropzone.addEventListener('drop', e => {
	e.preventDefault();
	dropzone.classList.remove('over');
	addFiles([...e.dataTransfer.files]);
});
fileInput.addEventListener('change', () => { addFiles([...fileInput.files]); fileInput.value = ''; });

function addFiles(newFiles) {
	newFiles.filter(f => f.type === 'application/pdf').forEach(f => files.push({ file: f, id: idCounter++ }));
	renderList();
}

function moveFile(fromIdx, toIdx) {
	if (toIdx < 0 || toIdx >= files.length) return;
	const [moved] = files.splice(fromIdx, 1);
	files.splice(toIdx, 0, moved);
	renderList();
}

function renderList() {
	fileListEl.innerHTML = '';
	const touch = isTouch();
	files.forEach((entry, idx) => {
		const item = document.createElement('div');
		item.className   = 'file-item' + (touch ? '' : ' draggable-desktop');
		item.draggable   = !touch;
		item.dataset.idx = idx;
		item.innerHTML = `
			<span class="file-icon">📄</span>
			<span class="file-name" title="${entry.file.name}">${entry.file.name}</span>
			<span class="file-size">${formatSize(entry.file.size)}</span>
			<div class="file-order-btns">
				<button class="file-order-btn btn-up"   aria-label="Monter">▲</button>
				<button class="file-order-btn btn-down" aria-label="Descendre">▼</button>
			</div>
			<button class="file-remove" aria-label="Supprimer">✕</button>`;

		item.querySelector('.file-remove').addEventListener('click', () => { files.splice(idx, 1); renderList(); });
		item.querySelector('.btn-up')    .addEventListener('click', () => moveFile(idx, idx - 1));
		item.querySelector('.btn-down')  .addEventListener('click', () => moveFile(idx, idx + 1));

		if (!touch) {
			item.addEventListener('dragstart', handleDragStart);
			item.addEventListener('dragover',  handleDragOver);
			item.addEventListener('drop',      handleDrop);
			item.addEventListener('dragend',   handleDragEnd);
		}
		fileListEl.appendChild(item);
	});

	dragHint.style.display = (!isTouch() && files.length > 1) ? 'block' : 'none';
	mergeBtn.disabled = files.length < 2;
	mergeDownload.style.display = 'none';
	mergeStatus.textContent = '';
	mergeStatus.className = 'status';
}

let dragSrcIdx = null;
function handleDragStart(e)  { dragSrcIdx = +this.dataset.idx; this.classList.add('dragging'); e.dataTransfer.effectAllowed = 'move'; }
function handleDragOver(e)   { e.preventDefault(); document.querySelectorAll('.file-item').forEach(el => el.classList.remove('drag-over')); this.classList.add('drag-over'); }
function handleDrop(e)       { e.preventDefault(); const t = +this.dataset.idx; if (dragSrcIdx !== null && dragSrcIdx !== t) { const [m] = files.splice(dragSrcIdx, 1); files.splice(t, 0, m); renderList(); } }
function handleDragEnd()     { document.querySelectorAll('.file-item').forEach(el => el.classList.remove('dragging', 'drag-over')); dragSrcIdx = null; }

mergeBtn.addEventListener('click', async () => {
	if (files.length < 2) return;
	mergeBtn.disabled = true;
	mergeDownload.style.display = 'none';
	mergeStatus.className = 'status';
	mergeStatus.innerHTML = '<span class="spinner"></span> Envoi en cours…';

	const formData = new FormData();
	files.forEach(e => formData.append('files', e.file, e.file.name));
	const ui = initProgress('merge-progress', '');

	try {
		const blob = await xhrUpload('/merge', formData, ui);
		mergeDownload.href = URL.createObjectURL(blob);
		mergeDownload.style.display = 'flex';
		mergeStatus.className = 'status ok';
		mergeStatus.textContent = '✓ Fusion réussie !';
	} catch (err) {
		mergeStatus.className = 'status error';
		mergeStatus.textContent = '✗ ' + err.message;
		hideProgress('merge-progress');
	} finally {
		mergeBtn.disabled = false;
	}
});

// ══════════════════════════════════════════════════════
// EXTRACTION
// ══════════════════════════════════════════════════════
const extractDropzone  = document.getElementById('extract-dropzone');
const extractFileInput = document.getElementById('extract-file-input');
const extractControls  = document.getElementById('extract-controls');
const extractPageInput = document.getElementById('extract-page-input');
const extractBtn       = document.getElementById('extract-btn');
const extractStatus    = document.getElementById('extract-status');
const extractDownload  = document.getElementById('extract-download');
const extractDropText  = document.getElementById('extract-dropzone-text');

let extractFile = null;

function setExtractFile(file) {
	if (!file || file.type !== 'application/pdf') return;
	extractFile = file;
	extractDropText.innerHTML = `<strong>${file.name}</strong><br><span style="color:var(--muted);font-size:.8rem">${formatSize(file.size)}</span>`;
	extractControls.style.display = 'block';
	extractBtn.disabled = false;
	extractStatus.textContent = '';
	extractStatus.className = 'status';
	extractDownload.style.display = 'none';
	extractPageInput.value = '1';
	hideProgress('extract-progress');
}

extractDropzone.addEventListener('dragover',  e => { e.preventDefault(); extractDropzone.classList.add('over'); });
extractDropzone.addEventListener('dragleave', () => extractDropzone.classList.remove('over'));
extractDropzone.addEventListener('drop', e => { e.preventDefault(); extractDropzone.classList.remove('over'); setExtractFile(e.dataTransfer.files[0]); });
extractFileInput.addEventListener('change', () => { setExtractFile(extractFileInput.files[0]); extractFileInput.value = ''; });

extractBtn.addEventListener('click', async () => {
	if (!extractFile) return;
	const pagesValue = extractPageInput.value.trim();
	if (!pagesValue) {
		extractStatus.className = 'status error';
		extractStatus.textContent = '✗ Spécifiez au moins une page.';
		return;
	}
	extractBtn.disabled = true;
	extractDownload.style.display = 'none';
	extractStatus.className = 'status';
	extractStatus.innerHTML = '<span class="spinner"></span> Envoi en cours…';

	const formData = new FormData();
	formData.append('file', extractFile, extractFile.name);
	formData.append('pages', pagesValue);
	const ui = initProgress('extract-progress', 'progress-fill-extract');

	try {
		const blob = await xhrUpload('/extract', formData, ui);
		const url = URL.createObjectURL(blob);
		extractDownload.href = url;
		extractDownload.download = extractFile.name.replace(/\.pdf$/i, '') + '_extrait.pdf';
		extractDownload.style.display = 'flex';
		extractStatus.className = 'status ok';
		extractStatus.textContent = '✓ Extraction réussie !';
	} catch (err) {
		extractStatus.className = 'status error';
		extractStatus.textContent = '✗ ' + err.message;
		hideProgress('extract-progress');
	} finally {
		extractBtn.disabled = false;
	}
});

// ══════════════════════════════════════════════════════
// ROTATION
// ══════════════════════════════════════════════════════
const rotateDropzone  = document.getElementById('rotate-dropzone');
const rotateFileInput = document.getElementById('rotate-file-input');
const rotateControls  = document.getElementById('rotate-controls');
const rotateListEl    = document.getElementById('rotate-list');
const rotateAddBtn    = document.getElementById('rotate-add-btn');
const rotateBtn       = document.getElementById('rotate-btn');
const rotateStatus    = document.getElementById('rotate-status');
const rotateDownload  = document.getElementById('rotate-download');
const rotateDropText  = document.getElementById('rotate-dropzone-text');

let rotateFile = null;
let rotateRows = [];

function setRotateFile(file) {
	if (!file || file.type !== 'application/pdf') return;
	rotateFile = file;
	rotateDropText.innerHTML = `<strong>${file.name}</strong><br><span style="color:var(--muted);font-size:.8rem">${formatSize(file.size)}</span>`;
	rotateControls.style.display = 'block';
	rotateStatus.textContent = '';
	rotateStatus.className = 'status';
	rotateDownload.style.display = 'none';
	rotateRows = [{ page: 1, angle: 90 }];
	hideProgress('rotate-progress');
	renderRotateRows();
}

function renderRotateRows() {
	rotateListEl.innerHTML = '';
	rotateRows.forEach((row, idx) => {
		const div = document.createElement('div');
		div.className = 'rotate-row';
		div.innerHTML = `
			<label>Page</label>
			<input type="number" class="rotate-page-input" min="1" value="${row.page}" />
			<div class="rotate-angle-btns">
				<button class="angle-btn ${row.angle === 90  ? 'active' : ''}" data-angle="90" >↻ 90°</button>
				<button class="angle-btn ${row.angle === 180 ? 'active' : ''}" data-angle="180">↻ 180°</button>
				<button class="angle-btn ${row.angle === 270 ? 'active' : ''}" data-angle="270">↺ 90°</button>
			</div>
			<button class="rotate-remove" aria-label="Supprimer">✕</button>`;

		div.querySelector('.rotate-page-input').addEventListener('input', e => {
			rotateRows[idx].page = Math.max(1, parseInt(e.target.value) || 1);
		});
		div.querySelectorAll('.angle-btn').forEach(btn => {
			btn.addEventListener('click', () => {
				rotateRows[idx].angle = parseInt(btn.dataset.angle);
				renderRotateRows();
			});
		});
		div.querySelector('.rotate-remove').addEventListener('click', () => {
			rotateRows.splice(idx, 1);
			renderRotateRows();
		});
		rotateListEl.appendChild(div);
	});
	rotateBtn.disabled = !rotateFile || rotateRows.length === 0;
}

rotateAddBtn.addEventListener('click', () => {
	const lastPage = rotateRows.length > 0 ? rotateRows[rotateRows.length - 1].page + 1 : 1;
	rotateRows.push({ page: lastPage, angle: 90 });
	renderRotateRows();
});

rotateDropzone.addEventListener('dragover',  e => { e.preventDefault(); rotateDropzone.classList.add('over'); });
rotateDropzone.addEventListener('dragleave', () => rotateDropzone.classList.remove('over'));
rotateDropzone.addEventListener('drop', e => { e.preventDefault(); rotateDropzone.classList.remove('over'); setRotateFile(e.dataTransfer.files[0]); });
rotateFileInput.addEventListener('change', () => { setRotateFile(rotateFileInput.files[0]); rotateFileInput.value = ''; });

rotateBtn.addEventListener('click', async () => {
	if (!rotateFile || rotateRows.length === 0) return;
	rotateBtn.disabled = true;
	rotateDownload.style.display = 'none';
	rotateStatus.className = 'status';
	rotateStatus.innerHTML = '<span class="spinner"></span> Envoi en cours…';

	const formData = new FormData();
	formData.append('file', rotateFile, rotateFile.name);
	formData.append('rotations', rotateRows.map(r => `${r.page}:${r.angle}`).join(','));
	const ui = initProgress('rotate-progress', 'progress-fill-rotate');

	try {
		const blob = await xhrUpload('/rotate', formData, ui);
		const url = URL.createObjectURL(blob);
		rotateDownload.href = url;
		rotateDownload.download = rotateFile.name.replace(/\.pdf$/i, '') + '_pivoté.pdf';
		rotateDownload.style.display = 'flex';
		rotateStatus.className = 'status ok';
		rotateStatus.textContent = '✓ Rotation appliquée !';
	} catch (err) {
		rotateStatus.className = 'status error';
		rotateStatus.textContent = '✗ ' + err.message;
		hideProgress('rotate-progress');
	} finally {
		rotateBtn.disabled = false;
	}
});

// ══════════════════════════════════════════════════════
// SUPPRESSION
// ══════════════════════════════════════════════════════
const deleteDropzone  = document.getElementById('delete-dropzone');
const deleteFileInput = document.getElementById('delete-file-input');
const deleteControls  = document.getElementById('delete-controls');
const deletePageInput = document.getElementById('delete-page-input');
const deleteBtn       = document.getElementById('delete-btn');
const deleteStatus    = document.getElementById('delete-status');
const deleteDownload  = document.getElementById('delete-download');
const deleteDropText  = document.getElementById('delete-dropzone-text');

let deleteFile = null;

function setDeleteFile(file) {
	if (!file || file.type !== 'application/pdf') return;
	deleteFile = file;
	deleteDropText.innerHTML = `<strong>${file.name}</strong><br><span style="color:var(--muted);font-size:.8rem">${formatSize(file.size)}</span>`;
	deleteControls.style.display = 'block';
	deleteBtn.disabled = false;
	deleteStatus.textContent = '';
	deleteStatus.className = 'status';
	deleteDownload.style.display = 'none';
	deletePageInput.value = '';
	hideProgress('delete-progress');
}

deleteDropzone.addEventListener('dragover',  e => { e.preventDefault(); deleteDropzone.classList.add('over'); });
deleteDropzone.addEventListener('dragleave', () => deleteDropzone.classList.remove('over'));
deleteDropzone.addEventListener('drop', e => { e.preventDefault(); deleteDropzone.classList.remove('over'); setDeleteFile(e.dataTransfer.files[0]); });
deleteFileInput.addEventListener('change', () => { setDeleteFile(deleteFileInput.files[0]); deleteFileInput.value = ''; });

deleteBtn.addEventListener('click', async () => {
	if (!deleteFile) return;
	const pagesValue = deletePageInput.value.trim();
	if (!pagesValue) {
		deleteStatus.className = 'status error';
		deleteStatus.textContent = '✗ Spécifiez au moins une page à supprimer.';
		return;
	}
	deleteBtn.disabled = true;
	deleteDownload.style.display = 'none';
	deleteStatus.className = 'status';
	deleteStatus.innerHTML = '<span class="spinner"></span> Envoi en cours…';

	const formData = new FormData();
	formData.append('file', deleteFile, deleteFile.name);
	formData.append('pages', pagesValue);
	const ui = initProgress('delete-progress', 'progress-fill-delete');

	try {
		const blob = await xhrUpload('/delete', formData, ui);
		const url = URL.createObjectURL(blob);
		deleteDownload.href = url;
		deleteDownload.download = deleteFile.name.replace(/\.pdf$/i, '') + '_modifié.pdf';
		deleteDownload.style.display = 'flex';
		deleteStatus.className = 'status ok';
		deleteStatus.textContent = '✓ Pages supprimées avec succès !';
	} catch (err) {
		deleteStatus.className = 'status error';
		deleteStatus.textContent = '✗ ' + err.message;
		hideProgress('delete-progress');
	} finally {
		deleteBtn.disabled = false;
	}
});

// ══════════════════════════════════════════════════════
// RÉORGANISATION
// ══════════════════════════════════════════════════════
const reorderDropzone  = document.getElementById('reorder-dropzone');
const reorderFileInput = document.getElementById('reorder-file-input');
const reorderControls  = document.getElementById('reorder-controls');
const reorderListEl    = document.getElementById('reorder-list');
const reorderBtn       = document.getElementById('reorder-btn');
const reorderStatus    = document.getElementById('reorder-status');
const reorderDownload  = document.getElementById('reorder-download');
const reorderDropText  = document.getElementById('reorder-dropzone-text');
const reorderPageCount = document.getElementById('reorder-page-count');
const reorderDragHint  = document.getElementById('reorder-drag-hint');

let reorderFile  = null;
let reorderPages = [];

function setReorderFile(file) {
	if (!file || file.type !== 'application/pdf') return;
	reorderFile = file;
	reorderDropText.innerHTML = `<strong>${file.name}</strong><br><span style="color:var(--muted);font-size:.8rem">${formatSize(file.size)}</span>`;
	hideProgress('reorder-progress');

	countPdfPages(file).then(n => {
		reorderPages = Array.from({ length: n }, (_, i) => ({ origPage: i + 1 }));
		reorderPageCount.textContent = `${n} page${n > 1 ? 's' : ''}`;
		reorderControls.style.display = 'block';
		reorderBtn.disabled = false;
		reorderStatus.textContent = '';
		reorderStatus.className = 'status';
		reorderDownload.style.display = 'none';
		renderReorderList();
	}).catch(() => {
		reorderStatus.className = 'status error';
		reorderStatus.textContent = '✗ Impossible de lire ce PDF.';
	});
}

function countPdfPages(file) {
	// Lit une tranche du fichier et la décode en latin-1 (safe pour le binaire PDF)
	function readSlice(blob) {
		return new Promise((res, rej) => {
			const r = new FileReader();
			r.onload  = e => res(new TextDecoder('latin1').decode(new Uint8Array(e.target.result)));
			r.onerror = rej;
			r.readAsArrayBuffer(blob);
		});
	}

	// Le noeud /Pages avec /Count est souvent en fin de fichier.
	// On lit 256 Ko au debut ET 256 Ko a la fin pour couvrir tous les cas.
	const CHUNK = 256 * 1024;
	return Promise.all([
		readSlice(file.slice(0, CHUNK)),
		readSlice(file.slice(Math.max(0, file.size - CHUNK))),
	]).then(([head, tail]) => {
		const matches = [...(head + tail).matchAll(/\/Count\s+(\d+)/g)];
		if (!matches.length) throw new Error('Structure PDF non reconnue');
		// Le plus grand /Count est toujours le total du document
		return Math.max(...matches.map(m => parseInt(m[1])));
	});
}

function moveReorderPage(fromIdx, toIdx) {
	if (toIdx < 0 || toIdx >= reorderPages.length) return;
	const [moved] = reorderPages.splice(fromIdx, 1);
	reorderPages.splice(toIdx, 0, moved);
	renderReorderList();
}

function renderReorderList() {
	reorderListEl.innerHTML = '';
	const touch = isTouch();

	reorderPages.forEach((page, idx) => {
		const card = document.createElement('div');
		card.className   = 'page-card' + (touch ? '' : ' draggable-desktop');
		card.draggable   = !touch;
		card.dataset.idx = idx;

		const isOriginalPos = page.origPage === idx + 1;
		card.innerHTML = `
			<div class="page-card-num">${idx + 1}</div>
			<span class="page-card-label">Page ${idx + 1}</span>
			<span class="page-card-orig">${isOriginalPos ? '' : `← orig. ${page.origPage}`}</span>
			<div class="page-card-order-btns">
				<button class="page-card-order-btn btn-up"   aria-label="Monter">▲</button>
				<button class="page-card-order-btn btn-down" aria-label="Descendre">▼</button>
			</div>`;

		card.querySelector('.btn-up')  .addEventListener('click', () => moveReorderPage(idx, idx - 1));
		card.querySelector('.btn-down').addEventListener('click', () => moveReorderPage(idx, idx + 1));

		if (!touch) {
			card.addEventListener('dragstart', reorderDragStart);
			card.addEventListener('dragover',  reorderDragOver);
			card.addEventListener('drop',      reorderDrop);
			card.addEventListener('dragend',   reorderDragEnd);
		}
		reorderListEl.appendChild(card);
	});

	reorderDragHint.style.display = (!isTouch() && reorderPages.length > 1) ? 'block' : 'none';
}

let reorderDragSrcIdx = null;
function reorderDragStart(e) { reorderDragSrcIdx = +this.dataset.idx; this.classList.add('dragging'); e.dataTransfer.effectAllowed = 'move'; }
function reorderDragOver(e)  { e.preventDefault(); document.querySelectorAll('.page-card').forEach(el => el.classList.remove('drag-over')); this.classList.add('drag-over'); }
function reorderDrop(e) {
	e.preventDefault();
	const t = +this.dataset.idx;
	if (reorderDragSrcIdx !== null && reorderDragSrcIdx !== t) {
		const [moved] = reorderPages.splice(reorderDragSrcIdx, 1);
		reorderPages.splice(t, 0, moved);
		renderReorderList();
	}
}
function reorderDragEnd() { document.querySelectorAll('.page-card').forEach(el => el.classList.remove('dragging', 'drag-over')); reorderDragSrcIdx = null; }

reorderDropzone.addEventListener('dragover',  e => { e.preventDefault(); reorderDropzone.classList.add('over'); });
reorderDropzone.addEventListener('dragleave', () => reorderDropzone.classList.remove('over'));
reorderDropzone.addEventListener('drop', e => { e.preventDefault(); reorderDropzone.classList.remove('over'); setReorderFile(e.dataTransfer.files[0]); });
reorderFileInput.addEventListener('change', () => { setReorderFile(reorderFileInput.files[0]); reorderFileInput.value = ''; });

reorderBtn.addEventListener('click', async () => {
	if (!reorderFile || reorderPages.length === 0) return;
	reorderBtn.disabled = true;
	reorderDownload.style.display = 'none';
	reorderStatus.className = 'status';
	reorderStatus.innerHTML = '<span class="spinner"></span> Envoi en cours…';

	const formData = new FormData();
	formData.append('file', reorderFile, reorderFile.name);
	formData.append('order', reorderPages.map(p => p.origPage).join(','));
	const ui = initProgress('reorder-progress', 'progress-fill-reorder');

	try {
		const blob = await xhrUpload('/reorder', formData, ui);
		const url = URL.createObjectURL(blob);
		reorderDownload.href = url;
		reorderDownload.download = reorderFile.name.replace(/\.pdf$/i, '') + '_réorganisé.pdf';
		reorderDownload.style.display = 'flex';
		reorderStatus.className = 'status ok';
		reorderStatus.textContent = '✓ Réorganisation appliquée !';
	} catch (err) {
		reorderStatus.className = 'status error';
		reorderStatus.textContent = '✗ ' + err.message;
		hideProgress('reorder-progress');
	} finally {
		reorderBtn.disabled = false;
	}
});
