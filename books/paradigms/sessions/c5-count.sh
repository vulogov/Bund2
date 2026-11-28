wc -l commands.bund functions.bund objects.bund rules.bund
diff commands.bund wool/commands.bund | grep -c '^[<>]'
diff functions.bund wool/functions.bund | grep -c '^[<>]'
diff objects.bund wool/objects.bund | grep -c '^[<>]'
diff rules.bund wool/rules.bund | grep -c '^[<>]'
