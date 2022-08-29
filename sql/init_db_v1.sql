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

CREATE TABLE `record` (
    `id` bigint unsigned NOT NULL AUTO_INCREMENT,
    `user_id` bigint unsigned NOT NULL,
    `uid` varchar(256) NOT NULL,
    `guid` varchar(256) NOT NULL,
    `mailbox` varchar(256) NOT NULL,
    `dt_sent` DATETIME NOT NULL,
    `tz_sent` DECIMAL(4) SIGNED NOT NULL,
    `dt_recv` DATETIME NOT NULL,
    `dt_saved` DATETIME NOT NULL,
    `size` bigint unsigned NOT NULL,
    `mail_to` varchar(256) NOT NULL,
    `mail_from` varchar(256) NOT NULL,
    `mail_subj` varchar(256) NOT NULL,
    UNIQUE (uid,mailbox),
    UNIQUE (guid),
    PRIMARY KEY (`id`),
    CONSTRAINT `fk_record_user_id`
        FOREIGN KEY (user_id) REFERENCES user (id)
            ON DELETE CASCADE
            ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `header` (
    `record_id` bigint unsigned NOT NULL,
    `seq` mediumint unsigned NOT NULL,
    `name` varchar(256) NOT NULL,
    `value` varchar(1024) NOT NULL,
    UNIQUE(record_id, seq),
    CONSTRAINT `fk_hdr_record_id`
        FOREIGN KEY (record_id) REFERENCES record (id)
        ON DELETE CASCADE
        ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `imap_flag` (
      `record_id` bigint unsigned NOT NULL,
      `name` varchar(256) NOT NULL,
      UNIQUE (record_id, name),
      CONSTRAINT `fk_if_record_id`
          FOREIGN KEY (record_id) REFERENCES record (id)
              ON DELETE CASCADE
              ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
