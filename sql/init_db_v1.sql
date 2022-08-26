CREATE TABLE `db_ver` (
    `version` mediumint unsigned NOT NULL COMMENT 'Database version',
    PRIMARY KEY (`version`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `user` (
    `id` bigint unsigned NOT NULL AUTO_INCREMENT,
    `user` varchar(128) NOT NULL,
    PRIMARY KEY (`id`),
    UNIQUE KEY `user` (`user`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

