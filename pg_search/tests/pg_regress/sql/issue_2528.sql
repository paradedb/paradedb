SELECT id, paradedb.score(id) FROM mock_items_issue_2528 WHERE description @@@ 'shoes' and in_stock = true LIMIT 5;
