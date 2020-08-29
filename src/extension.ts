import * as vscode from 'vscode';
import * as child_process from "child_process";

const EXTENSION_NAME = "VRLifeTime";
const TIMEOUT_TIME = 20;
let statusBar: vscode.StatusBarItem;


interface DetectorOutput {
	firstLock: {
		type: string,
		pos: string
	},
	secondLock: {
		type: string,
		pos: string,
	},
	callChain: string
}

// this method is called when vs code is activated
export function activate(context: vscode.ExtensionContext) {
	let lifetimeObj = {};
	const collection = vscode.languages.createDiagnosticCollection('result');
	let lastCallMillisec = Date.now();
	console.log(`${EXTENSION_NAME} is activated`);
	vscode.window.showInformationMessage(`${EXTENSION_NAME} is activated`);
	let selectedText = "";
	let outputChannel = vscode.window.createOutputChannel(`extension:${EXTENSION_NAME}`);
	let rootPath = "";
	if (vscode.workspace.workspaceFolders) {
		rootPath = vscode.workspace.workspaceFolders[0].uri.path;
	}
	let timeout1: NodeJS.Timer | undefined = undefined;



	// create a decorator type
	const lifetimeLineDecorationType = vscode.window.createTextEditorDecorationType({
		cursor: 'crosshair',
		// use a themable color. See package.json for the declaration and default values.
		backgroundColor: { id: 'vrlifetime.lifetimeLineBackground' },
		overviewRulerColor: "#ce0f0f38",
	 	overviewRulerLane: vscode.OverviewRulerLane.Left,
	});

	let activeEditor = vscode.window.activeTextEditor;
	if (!activeEditor) return;


	function stringifyPositionRange(select: vscode.Selection) {
		let pos1 = select.start;
		let pos2 = select.start;
		const editor = vscode.window.activeTextEditor;

		while(true){
			if (pos1.character == 0) break;
			pos2 = pos2.with({character: pos1.character - 1});
			let select2 = new vscode.Selection(pos2, pos1);
			if (!editor) break;
			let text = editor.document.getText(select2);
			if (text != "&") break;
			pos1 = pos2;
		}
		select.with({start:pos1});
				
		let s : string = "" + (select.start.line + 1) + ":" + (select.start.character + 1) +
			": " + (select.end.line + 1) + ":" + (select.end.character + 1);
		return s;
	}


	function parsePositionRangeString(s: string){
		let ranges: vscode.Range[] = [];
		let strings = s.split(", ")
		for (let i in strings) {
			let rangeString = strings[i].split(": ");
			let begin = rangeString[0].split(":");
			let beginPosition = new vscode.Position(Number(begin[0]) - 1, Number(begin[1]) - 1);
			let end = rangeString[1].split(":");
			let endPosition = new vscode.Position(Number(end[0]) - 1, Number(end[1]) - 1);
			let range = new vscode.Range(beginPosition, endPosition);
			ranges.push(range);
		}
		return ranges;
	}

	function getFunctionName(select: vscode.Selection){
		let pos = select.start;
		let line = pos.line;
		let regEx = /fn[ ]+(.+)\(/;
		const editor = vscode.window.activeTextEditor;
		if (!editor) return "";

		while (line >= 0){
			let textLine = editor.document.lineAt(line);
			let result = regEx.exec(textLine.text);
			if (result) {
				return result[1];
			}
			line -= 1;
		}
		return "";
	}

	function compileProject(){
		if (! vscode.workspace.workspaceFolders) return false;
		function isNightlyInstalled(){
			let stdout = "";
			let testProcess = child_process.spawnSync("rustup show",
			 {shell: true});
			
			stdout = testProcess.stdout.toString();
			outputChannel.appendLine(stdout);
			outputChannel.appendLine(testProcess.stderr.toString());
			let regEx = /nightly-2020-05-10/;
			let match = regEx.exec(stdout);
			if (match) return true;
			else return false;
		
		}

		if (!isNightlyInstalled()){
			statusBar.text = "installing rust nightly...";
			statusBar.show();
			// vscode.window.setStatusBarMessage();
			let process = child_process.spawnSync(
				'rustup toolchain install nightly-2020-05-10',
				{shell: true});
			outputChannel.appendLine(process.stdout.toString());
			outputChannel.appendLine(process.stderr.toString());
			statusBar.hide();

		}
		// statusBar.text = "building...";
		// statusBar.show();
		let process = child_process.spawnSync(`${__dirname}/../backend/lifetime_query/run.sh "${rootPath}"`,
				{shell: true});

		outputChannel.appendLine(process.stdout.toString());
		outputChannel.appendLine(process.stderr.toString());
		// statusBar.hide();
		return true;
	}


	function updateLifetimeObj(called: boolean){
		const editor = vscode.window.activeTextEditor;

		if (!editor) {
			return;
		}
		let select = editor.selection;
		let selectString = stringifyPositionRange(select);
		let fileRelativePath = vscode.workspace.asRelativePath(editor.document.uri.path);
		let inputObj = {
			root: rootPath,
			file: fileRelativePath,
			pos: selectString
		};
		let inputString = JSON.stringify(inputObj);
		outputChannel.appendLine("__dirname:" + __dirname);
		let process = child_process.spawnSync(
			`"${__dirname}/../backend/lifetime_query/query.sh" "${inputString.replace(/"/g, '\\"')}"`, 
			{shell: true});
		let returnMsg = process.stdout.toString() + process.stderr.toString();
		let returnObj = {};
		try {
			returnObj = JSON.parse(returnMsg);
		} catch(e){
			outputChannel.appendLine("Error:" + e);
			outputChannel.appendLine(returnMsg)
			if (!called) { 
				outputChannel.appendLine("trying to compile the project...");
				compileProject();
				updateLifetimeObj(true);
			}

		}
		// returnObj["src/main.rs"] = returnObj["src/main.rs"] + ", 45:46: 48:6" //!
		
		
		let str = `\nLifetime of ${selectedText} is:\n`;
		str += returnMsg;
		outputChannel.appendLine(str);
		for (let file in returnObj) {
			let s = returnObj[file];
			let ranges = parsePositionRangeString(s);
			returnObj[file] = ranges;
		}
		lifetimeObj = returnObj;
	}

	function updateDecorations() {
		const editor = vscode.window.activeTextEditor;
		const workspaceFolder = vscode.workspace.workspaceFolders;
		if (!workspaceFolder) return; 
		if (!editor) {
			return;
		}
		let select = editor.selection;
		let filename = editor.document.uri.path;
		let ranges: vscode.Range[] = [];
		for (let key in lifetimeObj){
			let regEx = new RegExp(key + "$");
			let match = regEx.exec(filename);
			if (match) {
				ranges = lifetimeObj[key];
				break;
			}
		}

		const lifetimeLines: vscode.DecorationOptions[] = [];
		if (ranges){
			for (let i in ranges) {
				
				const decoration = { range: ranges[i], hoverMessage: `lifetime for **${selectedText}**`};
				lifetimeLines.push(decoration);
			}
		}
		editor.setDecorations(lifetimeLineDecorationType, lifetimeLines);

	}
	//the inside function won't be triggered if the time between last time this function triggered and now is lower than time
	// function triggerWithLapse(callback: (...args: any[]) => void, ms: Number) {
	// 	let lastCallMillisec = Date.now();
	// 	setTimeout(() => {
			
	// 	}, timeout);
	// 	if (now - lastCallMillisec < ms) return;
	// 	millisec = now;
	// 	callback();
	// }

	function triggerUpdateDecorations() {
		if (timeout1) {
			clearTimeout(timeout1);
			timeout1 = undefined;
		}
		timeout1 = setTimeout(updateDecorations, TIMEOUT_TIME);
	}

	function parseDetectorOutput(s: string) {
		let lines = s.split("\n");
		// state:
		// 0: start
		// 1: first lock type
		// 2: first lock pos
		// 3: second lock type
		// 4: second lock pos
		//5: call chain
		let state = 0;
		let results = [];
		let result = Object();
		for (let i in lines) {
			let line = lines[i];
			if (state == 0) {
				if (line.startsWith("{")) {
					let regex = /FirstLock: \((\w+), "(.+)"\)/
					let matchObj = line.match(regex);
					if (matchObj) {
						result["firstLock"] = {type: `${matchObj[1]}<${matchObj[2]}>`}
						state = 1;
					}
				}
			}
			else if (state == 1) {
				let regex = /([^\t].*?):(\d+:\d+: \d+:\d+)/;
				let matchObj = line.match(regex);
				if (matchObj) {
					let fname = matchObj[1];
					let pos = matchObj[2];
					result["firstLock"].pos = pos;
					result["firstLock"].fname = fname;
					result["firstLock"].msg = "the other lock causing double-lock."
					state = 2;
				}
			}
			else if (state == 2) {
				let regex = /SecondLock: \((\w+), "(.+)"\)/
				let matchObj = line.match(regex);
				if (matchObj) {
					result["secondLock"] = {type: `${matchObj[1]}<${matchObj[2]}>`}
					state = 3;
				}
			}
			else if (state == 3) {
				let regex = /([^\t].*?):(\d+:\d+: \d+:\d+)/;
				let matchObj = line.match(regex);
				if (matchObj) {
					let fname = matchObj[1];
					let pos = matchObj[2];
					result["secondLock"].pos = pos;
					result["secondLock"].fname = fname;
					result["secondLock"].msg = "Potential double-locking bug.";
					state = 4;
				}
			}
			else if (state == 4) {
				let regex = /Callchains: \{(.+)\}/
				let matchObj = line.match(regex);
				if (matchObj) {
					result["firstLock"].msg += " Call chain: " + matchObj[1];
					state = 0;
					results.push(result);
					result = Object();
				}
			}
		}
		return results;
	}


	function getDiagnosticObj() {
		let analyzerInputObj = {
		}
		let process = child_process.spawnSync(
			`cd ${rootPath} && cargo clean && cargo +nightly-2020-05-10 lock-bug-detect double-lock`, 
			{shell: true});
		let returnMsg = process.stdout.toString();
		let detectorOutput = Object();
		try {
			detectorOutput = parseDetectorOutput(returnMsg);
		} catch(e){
			outputChannel.appendLine("Error:" + e);
			return;
		}
		// diagnosticObj = {
		// 	filename1: {
		// 		positionRange1: 
		//      [{
		//			msg: Error_message_string,
		//          related: 
		//			[
					// 	{
					// 		fname: str,
					// 		pos: str,
					// 		msg: str,
					// 	},
					// 	...

					// ]
		// 		positionRange2: [...],
		// 		...
		// 	}],
		// 	filename2: {...},
		// 	...
		// }
		let diagnosticObj = Object();
		const SWITCH = 1;
		const POS_KEY = 1;

		for (let i in detectorOutput) {
			let key_elem = detectorOutput[i]["secondLock"];
			let related_elem = detectorOutput[i]["firstLock"];
			if (key_elem["fname"] in diagnosticObj) {
				if (key_elem["pos"] in diagnosticObj[key_elem["fname"]]) {
					diagnosticObj[key_elem["fname"]][key_elem["pos"]].push({
						msg: key_elem["msg"],
						related: [related_elem]
					});
				}
				else {
					diagnosticObj[key_elem["fname"]][key_elem["pos"]] = [{
						msg: key_elem["msg"],
						related: [related_elem]
					}]
				}
			} 
			else {
				diagnosticObj[key_elem["fname"]] = Object();
				diagnosticObj[key_elem["fname"]][key_elem["pos"]] = [{
					msg: key_elem["msg"],
					related: [related_elem]
				}];
			}
		}

		return diagnosticObj;

	}


	function updateDiagnostics(collection: vscode.DiagnosticCollection): void {
		let editor = vscode.window.activeTextEditor;
		if (!editor || !vscode.workspace.workspaceFolders) return;
		let diagnosticObj = getDiagnosticObj();
		let diagnosticArray = [];
		let document = editor.document;
		let dirPath = vscode.workspace.workspaceFolders[0].uri.path;

		for (let filename in diagnosticObj) {
			let filePath = dirPath + '/' + filename;
			let fileUri = vscode.Uri.file(filePath);
			let fileDiagnostic = diagnosticObj[filename];
			let entryArray = []
			for (let range in fileDiagnostic) {
				for (let i in fileDiagnostic[range]) {
					let relatedInformations = []
					for (let j in fileDiagnostic[range][i]["related"]) {
						let relatedInfo = fileDiagnostic[range][i]["related"][j];
						let relatedUri = vscode.Uri.file(dirPath + "/" + relatedInfo["fname"]);
						let posRange = parsePositionRange(relatedInfo["pos"]);
						relatedInformations.push(
							new vscode.DiagnosticRelatedInformation(
								new vscode.Location(
									relatedUri,
									posRange),
									relatedInfo["msg"]
							)
						);
					}
					entryArray.push({
						code: '',
						message: fileDiagnostic[range][i]["msg"],
						range: parsePositionRange(range),
						severity: vscode.DiagnosticSeverity.Warning,
						source: `${EXTENSION_NAME}`,
						relatedInformation: relatedInformations
					});
				}

			}
			diagnosticArray.push([
				fileUri, entryArray
			]);

		}
		if (document && document.uri.path.search(dirPath) != -1) {
			collection.set(diagnosticArray);
		} else {
			collection.clear();
		}
	}


	vscode.window.onDidChangeActiveTextEditor(editor => {
		activeEditor = editor;
		if (editor) {
			// triggerUpdateDecorationsAndInfoDiagnostic();
			triggerUpdateDecorations();
			updateDiagnostics(collection);

		}
	}, null, context.subscriptions);

	vscode.workspace.onDidChangeTextDocument(event => {
		if (activeEditor && event.document === activeEditor.document) {
			triggerUpdateDecorations();


		}
	}, null, context.subscriptions);

	vscode.workspace.onDidChangeWorkspaceFolders(event => {
		if (vscode.workspace.workspaceFolders) {
			rootPath = vscode.workspace.workspaceFolders[0].uri.path;
		}
	}, null, context.subscriptions);


	vscode.window.onDidChangeTextEditorSelection(event => {
		if (activeEditor == event.textEditor) {
			if (event.selections[0].isEmpty) return;
			selectedText = activeEditor.document.getText(event.selections[0]);
			outputChannel.appendLine("selection changed 1");
			updateLifetimeObj(false);
			outputChannel.appendLine("selection changed");
			triggerUpdateDecorations();
		}
	}, null, context.subscriptions);
	

	vscode.workspace.onDidSaveTextDocument(event => {
		compileProject();
		updateDiagnostics(collection);
	}, null, context.subscriptions);

	if (activeEditor) {
		updateDiagnostics(collection);

	}	

}


function parsePosition(s: string) {
	let result = s.split(":")
	return new vscode.Position(Number(result[0]) - 1, Number(result[1]) - 1)
}

function parsePositionRange(s: string) {
	let tmp = s.split(": ");
	return new vscode.Range(
			parsePosition(tmp[0]),
			parsePosition(tmp[1])
		);
}
