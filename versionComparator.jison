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

%start ruleset

%%

// A valid ruleset is either an asterisk or an expression
ruleset :
	'*' EOF { return "return true;"; }
|	E EOF {
		return "$v = $v || \"\"; \
			$vv = $v.replace(/[ _]/g, '.').replace(/a(?:lpha)/i, -3).replace(/b(?:eta)?/i, -2).replace(/d(?:ev)?/i, -4).replace(/rc/i, -1).replace(/pl/i, 1).split(/\\./).map(function (item) { \
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

// A valid version is either $v or a valid version number
V : 
	'$v' { $$ = '$vv'; }
|	VERSION {
		var version = $1.replace(/[ _]/g, '.').replace(/a(?:lpha)/i, -3).replace(/b(?:eta)?/i, -2).replace(/d(?:ev)?/i, -4).replace(/rc/i, -1).replace(/pl/i, 1).split(/\./).map(function (item) { 
			return parseInt(item, 10); 
		});
		
		for (var i = 0; i < 5; i++) if (version[i] == null) version[i] = 0;
		$$ = JSON.stringify(version);
	}
;

// A valid expression is either a chain of ANDs, a chain of ORs or a valid subexpression
E : AND | OR | e ;

// A valid chain of ANDs is a subexpression followed by &&, followed by either a chain of ANDs or a subexpression
AND :
	e '&&' e { $$ = '(' + $1 + $2 + $3 + ')'; }
|	e '&&' AND { $$ = '(' + $1 + $2 + $3 + ')'; }
;

// A valid chain of ORs is a subexpression followed by ||, followed by either a chain of ORs or a subexpression
OR :
	e '||' e { $$ = '(' + $1 + $2 + $3 + ')'; }
|	e '||' OR { $$ = '(' + $1 + $2 + $3 + ')'; }
;

// A valid subexpression is either an expression in parentheses or a comparison
e : 
	'(' E ')' { $$ = $1 + $2 + $3; }
|	'$v' '!~' WORD { $$ = '(!/' + $3 + '/i.test(' + $1 + '))'; }
|	'$v' '~' WORD { $$ = '(/' + $3 + '/i.test(' + $1 + '))'; }
|	V '!=' V  { $$ = '(helper(' + $1 + ',' + $3 + ')' + $2 + '0)'; }
|	V RELATION V RELATION V { $$ = '(helper(' + $1 + ',' + $3 + ')' + $2 + '0)&&(helper(' + $3 + ',' + $5 + ')' + $4 + '0)'; }
|	V RELATION V { $$ = '(helper(' + $1 + ',' + $3 + ')' + $2 + '0)'; }
|	V '=' V  { $$ = '(helper(' + $1 + ',' + $3 + ')==0)'; }
;

RELATION : '<' | '>' | '<=' | '>=' ;
