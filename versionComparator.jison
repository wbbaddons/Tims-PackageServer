%lex
%options case-insensitive flex
%%

"&&"	return '&&'
"||"	return '||'
"("	return '('
")"	return ')'
">="	return '>='
"<="	return '<='
"!="	return '!='
"!~"	return '!~'
">"	return '>'
"<"	return '<'
"=="	return '='
"="	return '='
"~"	return '~'
"*"	return '*'
"$v"	return '$v'
[0-9]+\.[0-9]+\.[0-9]+(?:\s(?:a|alpha|b|beta|d|dev|rc|pl)\s[0-9]+)? return 'VERSION'
(a(?:lpha)?|b(?:eta)|d(?:dev)|rc|pl) return 'WORD'
\s+	/* skip */
<<EOF>>	return 'EOF'
.	return 'INVALID'

/lex

%left '&&' '||'
%left '='
%left '<=' '>=' '=' '<' '>' '!=' '!~' '~'

%start expressions

%%

expressions :
	  '*' EOF { return "return true;" }
	| e EOF {
		return "$vv = $v.replace(/[ _]/g, '.').replace(/a(?:lpha)/i, -3).replace(/b(?:eta)?/i, -2).replace(/d(?:ev)?/i, -4).replace(/rc/i, -1).replace(/pl/i, 1).split(/\\./).map(function (item) { \
				return parseInt(item, 10); \
			}); \
			for (var i = 0; i < 5; i++) if ($vv[i] == null) $vv[i] = 0; \
			var helper = function helper(v1, v2) { \
				for (var i = 0; i < 5; i++) { \
					if ((parseInt(v1[i])) === parseInt(v2[i])) continue; \
					if ((parseInt(v1[i])) < parseInt(v2[i])) return -1; \
					if ((parseInt(v1[i])) > parseInt(v2[i])) return 1; \
				} \
				return 0; \
			}; \
			return " + $1 + ";"
	}
;

V : 
	  '$v' { $$ = '$vv' }
	| VERSION { 
		var version = $1.replace(/[ _]/g, '.').replace(/a(?:lpha)/i, -3).replace(/b(?:eta)?/i, -2).replace(/d(?:ev)?/i, -4).replace(/rc/i, -1).replace(/pl/i, 1).split(/\./).map(function (item) { 
			return parseInt(item, 10); 
		});
		
		for (var i = 0; i < 5; i++) if (version[i] == null) version[i] = 0;
		$$ = JSON.stringify(version);
	}
;

RELATION : '<' | '>' | '<=' | '>=' ;

e : 
	  '(' e ')' { $$ = $1 + $2 + $3 }
	| e '&&' e { $$ = '(' + $1 + $2 + $3 + ')' }
	| e '||' e { $$ = '(' + $1 + $2 + $3 + ')' }
	| '$v' '!~' WORD { $$ = '(!/' + $3 + '/i.test(' + $1 + '))' }
	| '$v' '~' WORD { $$ = '(/' + $3 + '/i.test(' + $1 + '))' }
	| V '!=' V  { $$ = '(helper(' + $1 + ',' + $3 + ')' + $2 + '0)' }
	| V RELATION V RELATION V { $$ = '(helper(' + $1 + ',' + $3 + ')' + $2 + '0)&&(helper(' + $3 + ',' + $5 + ')' + $4 + '0)' }
	| V RELATION V { $$ = '(helper(' + $1 + ',' + $3 + ')' + $2 + '0)' }
	| V '=' V  { $$ = '(helper(' + $1 + ',' + $3 + ')==0)' }
;
