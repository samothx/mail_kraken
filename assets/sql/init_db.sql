CREATE TABLE `db_ver` (
    `version` int(11) NOT NULL COMMENT 'Database version',
    PRIMARY KEY (`version`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `user` (
    `id` bigint(20) NOT NULL AUTO_INCREMENT,
    `user` varchar(128) NOT NULL,
    `passwd` varchar(128) NOT NULL,
    PRIMARY KEY (`id`),
    UNIQUE KEY `user` (`user`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
